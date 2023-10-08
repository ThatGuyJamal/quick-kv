use std::collections::{BTreeSet, VecDeque};
use std::fs::{File, OpenOptions};
use std::sync::{Arc, Mutex};
use std::time::{Instant, Duration};
use std::{fmt::Debug, hash::Hash};

use anyhow::Ok;
use bytes::Bytes;
use log::LevelFilter;
use serde::Serialize;
use serde::de::DeserializeOwned;
use simple_logger::SimpleLogger;
use time::macros::format_description;

use self::config::DatabaseConfiguration;
use crate::types::HashMap;

pub(crate) mod batcher;
pub(crate) mod config;
pub(crate) mod runtime;

#[derive(Debug)]
pub(crate) struct State
{
    /// The key-value store entries in memory
    pub(crate) entries: HashMap<String, Entry>,

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
pub(crate) struct Entry
{
    /// Stored data
    pub(crate) data: Bytes,
    /// Instant at which the entry expires and should be removed from the
    /// database.
    pub(crate) expires_at: Option<Instant>,
}

/// The database consumed by clients.
///
/// Controls the state of the data-store and the background task.
#[derive(Debug)]
pub(crate) struct Database<'a, S>
where
    S: Serialize + DeserializeOwned + Debug + Eq + PartialEq + Hash + Send + Sync,
{
    state: Arc<Mutex<State>>,
    config: &'a DatabaseConfiguration,
    /// Jobs to be processed by the background task. 
    /// 
    /// todo - implement background task
    background_jobs: VecDeque<()>,
}

impl<'a, S> Database<'a, S>
where
    S: Serialize + DeserializeOwned + Debug + Eq + PartialEq + Hash + Send + Sync,
{
    pub(crate) fn new(config: DatabaseConfiguration) -> anyhow::Result<Self>
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
            .open(&config.path.unwrap_or_default())?;

        log::info!("QuickSchemaClient Initialized!");

        Ok(Self {
            state: Arc::new(Mutex::new(State {
                entries: HashMap::default(),
                expirations: BTreeSet::default(),
                file,
            })),
            config: &config,
            background_jobs: VecDeque::new(),
        })
    }

    pub(crate) fn get(&mut self, key: String) -> anyhow::Result<Option<S>>
    {
        log::info!("[GET] Searching for key: {}", key);

            let state = self.state.lock()?;
            if let Some(entry) = state.entries.get(&key) {
                log::info!("[GET] Found key: {}", key);
                return Ok(Some(bincode::deserialize(&entry.data)?));
            }

        Ok(None)
    }

    pub(crate) fn set(&mut self, key: &str, value: S, ttl: Option<Duration>) -> anyhow::Result<()>
    {
        log::info!("[SET] Setting key: {}", key);

        // First check if the data already exists; if so, update it instead
        let mut state = self.state.lock()?;

        // If this `set` becomes the key that expires **next**, the background
        // task needs to be notified so it can update its state.
        //
        // Whether or not the task needs to be notified is computed during the
        // `set` routine.
        let mut notify = false;

        let mut expires_at = ttl.map(| d | {
            // `Instant` at which the key expires.
            let when = Instant::now() + d;

            // Only notify the worker task if the newly inserted expiration is the
            // **next** key to evict. In this case, the worker needs to be woken up
            // to update its state.
            notify = state.

            // Track the expiration.
            state.expirations.insert((when, key));
            when
        });

        let entry = state.entries.insert(
            &key,
            Entry {
                data: bincode::serialize(&value)?,
                expires_at,
            },
        );

        if let Some(e) = entry {
            if let Some(when) = e.expires_at {
                // Remove the old expiration.
                state.expirations.remove(&(when, &key));
            }
        }

        drop(state);

        if notify {
            self.background_jobs.push_back(());
        }

        // todo - write to disk (if disk runtime)

        Ok(())
    }

     pub(crate) fn update(&mut self, key: &str, value: S) -> anyhow::Result<()>
     {
        todo!()
     }

     pub(crate) fn delete(&mut self, key: &str) -> anyhow::Result<()>
     {
        todo!()
     }
}
