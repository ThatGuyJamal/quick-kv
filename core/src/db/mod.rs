use std::collections::BTreeSet;
use std::fmt::Debug;
use std::fs::{File, OpenOptions};
use std::hash::Hash;
use std::sync::{mpsc, Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};

use anyhow::{Ok, Result};
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
    pub(crate) expirations: BTreeSet<(Instant, String)>,
    pub(crate) file: File,
}

/// Entry in the key-value store
#[derive(Debug)]
pub(crate) struct Entry<T>
where
    T: Serialize + DeserializeOwned + Debug + Eq + PartialEq + Hash + Send + Sync + Clone,
{
    /// Stored data
    pub(crate) data: T,
    /// Instant at which the entry expires and should be removed from the
    /// database.
    pub(crate) expires_at: Option<Instant>,
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
                            let now = Instant::now();
                            let mut state = state_clone.lock().unwrap();

                            // Iterate over keys and check TTL.
                            let mut keys_to_remove = vec![];

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
                file,
            })),
            config,
            ttl_manager: ttl_sender,
        })
    }

    pub(crate) fn get(&mut self, key: String) -> Result<Option<S>>
    {
        log::info!("[GET] Searching for key: {}", key);

        let mut state = self.state.lock().unwrap();

        if let Some(entry) = state.entries.get(&key) {
            if let Some(expires_at) = entry.expires_at {
                if expires_at < Instant::now() {
                    // The key has expired. Remove it from the database.
                    state.entries.remove(&key);
                    state.expirations.remove(&(expires_at, key));
                    return Ok(None);
                }
            }

            return Ok(Some(entry.data.clone()));
        }

        Ok(None)
    }

    pub(crate) fn set(&mut self, key: String, value: S, ttl: Option<Duration>) -> Result<()>
    {
        log::info!("[SET] Setting key: {}", key);

        // First check if the data already exists; if so, update it instead
        let mut state = self.state.lock().unwrap();

        let entry = state.entries.insert(
            key.clone(),
            Entry {
                data: value,
                expires_at: None,
            },
        );

        if let Some(e) = entry {
            if let Some(when) = e.expires_at {
                // Remove the old expiration.
                state.expirations.remove(&(when, key));
            }
        }

        drop(state);

        // todo - write to disk (if disk runtime)

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
