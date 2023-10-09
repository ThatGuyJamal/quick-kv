use std::collections::BTreeSet;
use std::fmt::Debug;
use std::fs::{File, OpenOptions};
use std::hash::Hash;
use std::io::{BufReader, BufWriter, Write};
use std::sync::{mpsc, Arc, Mutex, MutexGuard};
use std::thread;
use std::time::Duration;

use anyhow::{Ok, Result};
use chrono::{DateTime, Utc};
use log::LevelFilter;
use serde::de::DeserializeOwned;
use serde::Serialize;
use simple_logger::SimpleLogger;
use time::macros::format_description;

use self::config::DatabaseConfiguration;
use crate::types::HashMap;

pub(crate) mod batcher;
pub(crate) mod config;
pub(crate) mod runtime;

#[derive(Debug)]
pub(crate) struct State<T>
where
    T: Serialize + DeserializeOwned + Debug + Eq + PartialEq + Hash + Send + Sync + Clone,
{
    /// The key-value store entries in memory
    pub(crate) entries: HashMap<String, Entry<T>>,

    /// Tracks key TTLs.
    ///
    /// A `BTreeSet` is used to maintain expirations sorted by when they expire.
    /// This allows the background task to iterate this map to find the value
    /// expiring next.
    ///
    /// While highly unlikely, it is possible for more than one expiration to be
    /// created for the same instant. Because of this, the `Instant` is
    /// insufficient for the key. A unique key (`String`) is used to
    /// break these ties.
    pub(crate) expirations: BTreeSet<(DateTime<Utc>, String)>,
}

/// Entry in the key-value store
#[derive(Debug, Serialize, Clone)]
pub(crate) struct Entry<T>
where
    T: Serialize + DeserializeOwned + Debug + Eq + PartialEq + Hash + Send + Sync,
{
    /// Stored data
    pub(crate) data: T,
    /// Instant at which the entry expires and should be removed from the
    /// database.
    pub(crate) expires_at: Option<DateTime<Utc>>,
}

impl<T> Entry<T>
where
    T: Serialize + DeserializeOwned + Debug + Eq + PartialEq + Hash + Send + Sync,
{
    pub(crate) fn new(data: T, expires_at: Option<DateTime<Utc>>) -> Self
    {
        Self { data, expires_at }
    }
}

/// Signals sent to the background task
pub(super) enum TTLSignal
{
    Check,
    Exit,
}

/// The database consumed by clients.
///
/// Controls the state of the data-store and the background task.
#[derive(Debug)]
pub(crate) struct Database<'a, S>
where
    S: Serialize + DeserializeOwned + Debug + Eq + PartialEq + Hash + Send + Sync + Clone + 'static,
{
    pub(crate) state: Arc<Mutex<State<S>>>,
    pub(super) config: &'a DatabaseConfiguration,
    pub(super) ttl_manager: mpsc::Sender<TTLSignal>,
    pub(super) writer: Arc<Mutex<BufWriter<File>>>,
    pub(super) reader: Arc<Mutex<BufReader<File>>>,
}

impl<'a, S> Database<'a, S>
where
    S: Serialize + DeserializeOwned + Debug + Eq + PartialEq + Hash + Send + Sync + Clone + 'static,
{
    pub(crate) fn new(&self, config: &'a DatabaseConfiguration) -> Result<Self>
    {
        if config.log.unwrap_or_default() {
            SimpleLogger::new()
                .with_colors(true)
                .with_level(config.log_level.unwrap_or(LevelFilter::Info))
                .with_timestamp_format(format_description!("[year]-[month]-[day] [hour]:[minute]:[second]"))
                .init()?;
        }

        let file = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .open(config.path.as_ref().unwrap())?;

        let file_clone = file.try_clone()?;
        let file_clone2 = file.try_clone()?;

        // Create a channel for TTL check signals.
        let (ttl_sender, ttl_receiver) = mpsc::channel::<TTLSignal>();

        {
            // Spawn a background thread to manage TTL expiration.
            let state_clone = self.state.clone();
            thread::spawn(move || {
                log::debug!("[Bootstrap] Starting background task");

                loop {
                    let signal = ttl_receiver.recv().unwrap();

                    match signal {
                        TTLSignal::Check => {
                            // TTL check signal received, perform TTL checks here.
                            let now = Utc::now();
                            let mut state: MutexGuard<'_, State<S>> = state_clone.lock().unwrap();

                            // Iterate over keys and check TTL.
                            let mut keys_to_remove = Vec::new();

                            for (key, entry) in state.entries.iter_mut() {
                                if let Some(expires_at) = entry.expires_at {
                                    if expires_at <= now {
                                        // Key has expired, mark it for removal.
                                        keys_to_remove.push(key.clone());
                                    }
                                }
                            }

                            // Remove expired keys.
                            for key in keys_to_remove {
                                state.entries.remove(&key);
                                state.expirations.remove(&(now, key));
                            }
                        }
                        TTLSignal::Exit => {
                            log::warn!("[Bootstrap] Received exit signal, exiting background task");
                            break;
                        }
                    }
                }
            });
        }

        log::info!("[Bootstrap] QuickSchemaClient Initialized!");

        Ok(Self {
            state: Arc::new(Mutex::new(State {
                entries: HashMap::default(),
                expirations: BTreeSet::default(),
            })),
            config,
            ttl_manager: ttl_sender,
            writer: Arc::new(Mutex::new(BufWriter::new(file_clone))),
            reader: Arc::new(Mutex::new(BufReader::new(file_clone2))),
        })
    }

    pub(crate) fn get(&mut self, key: String) -> Result<Option<S>>
    {
        log::debug!("[GET] Searching for key: {}", key);

        self.ttl_manager.send(TTLSignal::Check)?;

        let state = self.state.lock().unwrap();

        if let Some(entry) = state.entries.get(&key) {
            log::debug!("[GET] Found key: {}", key);
            return Ok(Some(entry.data.clone()));
        }

        Ok(None)

        // Maybe we will check file, if no cache is found. Although for now this should
        // Never happen so we will just return None if nothing is found.
    }

    pub(crate) fn set(&mut self, key: String, value: S, ttl: Option<Duration>) -> Result<()>
    {
        log::info!("[SET] Setting key: {}", key);

        self.ttl_manager.send(TTLSignal::Check)?;

        // First check if the data already exists; if so, update it instead
        let mut state = self.state.lock().unwrap();

        let expires_at: Option<DateTime<Utc>> = if let Some(ttl) = ttl {
            Some(Utc::now() + chrono::Duration::from_std(ttl)?)
        } else {
            None
        };

        // Build the entry
        let entry = Entry::new(value, expires_at);

        // Set the entry in the state
        state.entries.insert(key.clone(), entry.clone());

        // Serialize the entry and write it to the file
        let mut w = self.writer.lock().unwrap();
        w.write_all(&bincode::serialize(&entry)?)?;

        // Flush the writer and sync the file
        w.flush()?;
        w.get_ref().sync_all()?;

        log::info!("[SET] Key set: {}", key);

        Ok(())
    }

    pub(crate) fn update(&mut self, key: &str, value: S) -> Result<()>
    {
        todo!()
    }

    pub(crate) fn delete(&mut self, key: &str) -> Result<()>
    {
        todo!()
    }
}
