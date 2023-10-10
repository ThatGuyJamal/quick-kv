use std::collections::BTreeSet;
use std::fmt::Debug;
use std::fs::{File, OpenOptions};
use std::hash::Hash;
use std::io::{self, BufReader, BufWriter, Seek, SeekFrom, Write};
use std::sync::{mpsc, Arc, Mutex, MutexGuard};
use std::thread;
use std::time::Duration;

use chrono::{DateTime, Utc};
use log::LevelFilter;
use serde::de::DeserializeOwned;
use serde::{Deserialize, Deserializer, Serialize};
use simple_logger::SimpleLogger;
use time::macros::format_description;

use self::config::DatabaseConfiguration;
use self::runtime::RuntTimeType;
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
    pub(crate) key: String,
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
    pub(crate) fn new(key: String, data: T, expires_at: Option<DateTime<Utc>>) -> Self
    {
        Self { key, data, expires_at }
    }
}

impl<'de, T> Deserialize<'de> for Entry<T>
where
    T: Serialize + DeserializeOwned + Debug + Eq + PartialEq + Hash + Send + Sync,
{
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        #[derive(Deserialize)]
        struct EntryHelper<T>
        {
            key: String,
            data: T,
            expires_at: Option<DateTime<Utc>>,
        }

        let helper = EntryHelper::<T>::deserialize(deserializer)?;

        Ok(Self {
            key: helper.key,
            data: helper.data,
            expires_at: helper.expires_at,
        })
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
    pub(crate) fn new(config: &'a DatabaseConfiguration) -> anyhow::Result<Self>
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
            .append(true)
            .create(true)
            .open(config.path.as_ref().unwrap())?;

        // Create two clones of the file handle, one for reading and one for writing.
        let file_clone = file.try_clone()?;
        let file_clone2 = file.try_clone()?;

        // Create a channel for TTL check signals.
        let (ttl_sender, ttl_receiver) = mpsc::channel::<TTLSignal>();

        // To access our state from the background task, we need to initialize it first,
        // so we create this wrapper struct to hold it and after we send it back to Self.
        let output = Self {
            state: Arc::new(Mutex::new(State {
                entries: HashMap::default(),
                expirations: BTreeSet::default(),
            })),
            config,
            ttl_manager: ttl_sender,
            writer: Arc::new(Mutex::new(BufWriter::new(file_clone))),
            reader: Arc::new(Mutex::new(BufReader::new(file_clone2))),
        };

        let state_clone = output.state.clone();

        {
            // Spawn a background thread to manage TTL expiration.
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

        Ok(output)
    }

    pub(crate) fn get(&mut self, key: String) -> anyhow::Result<Option<S>>
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

    pub(crate) fn set(&mut self, key: &str, value: S, ttl: Option<Duration>) -> anyhow::Result<()>
    {
        log::info!("[SET] Setting key: {}", key);

        self.ttl_manager.send(TTLSignal::Check)?;

        // First check if the data already exists; if so, update it instead
        let mut state = self.state.lock().unwrap();

        let expires_at: Option<DateTime<Utc>> = self.get_ttl(ttl)?;

        // Build the entry
        let entry = Entry::new(key.to_string(), value, expires_at);

        // Set the entry in the state
        state.entries.insert(key.to_string(), entry.clone());

        if self.is_disk_runtime() {
            // Serialize the entry and write it to the file
            let mut w: MutexGuard<'_, BufWriter<File>> = self.writer.lock().unwrap();

            w.seek(SeekFrom::End(0))?; // Seek to the end of the file (append)
            w.write_all(&bincode::serialize(&entry)?)?;

            // Flush the writer and sync the file
            w.flush()?;
            w.get_ref().sync_all()?;
        }

        log::info!("[SET] Key set: {}", key);

        Ok(())
    }

    pub(crate) fn update(&mut self, key: &str, value: S, ttl: Option<Duration>, upsert: Option<bool>) -> anyhow::Result<()>
    {
        log::info!("[UPDATE] Attempting {} update...", key);

        self.ttl_manager.send(TTLSignal::Check)?;

        let mut state = self.state.lock().unwrap();

        if !state.entries.contains_key(key) {
            log::debug!("[UPDATE] Key not found: {}", key);
            return Ok(());
        }

        let upsert = upsert.unwrap_or_else(|| false);

        if !upsert {
            log::debug!("[UPDATE] Upsert is disabled, skipping set attempt...");
            return Ok(());
        }

        state
            .entries
            .insert(key.to_string(), Entry::new(key.to_string(), value.clone(), None));

        if self.is_disk_runtime() {
            let mut r = self.reader.lock().unwrap();

            r.seek(SeekFrom::Start(0))?;

            let mut updated_bytes = Vec::new();

            loop {
                match bincode::deserialize_from::<_, Entry<S>>(&mut r.get_mut()) {
                    Ok(entry) => {
                        if key == entry.key {
                            let _ttl = self.get_ttl(ttl)?;
                            let new_entry = Entry::new(key.to_string(), value.clone(), _ttl);
                            updated_bytes.push(new_entry);
                        } else {
                            updated_bytes.push(entry)
                        }
                    }
                    Err(e) => {
                        if let bincode::ErrorKind::Io(io_err) = e.as_ref() {
                            if io_err.kind() == io::ErrorKind::UnexpectedEof {
                                // Reached the end of the serialized data
                                break;
                            } else {
                                return Err(e.into());
                            }
                        }
                    }
                }
            }

            drop(r);

            let mut w = self.writer.lock().unwrap();

            w.seek(SeekFrom::Start(0))?;

            for entry in updated_bytes {
                w.write_all(&bincode::serialize(&entry)?)?;
            }

            w.flush()?;
            w.get_ref().sync_all()?;
        }

        log::info!("[UPDATE] Key updated: {}", key);

        Ok(())
    }

    pub(crate) fn delete(&mut self, key: &str) -> anyhow::Result<()>
    {
        log::info!("[DELETE] Deleting key: {}", key);

        self.ttl_manager.send(TTLSignal::Check)?;

        let mut state = self.state.lock().unwrap();

        if !state.entries.contains_key(key) {
            log::debug!("[DELETE] Key not found: {}", key);
            return Ok(());
        }

        state.entries.remove(key);

        if self.is_disk_runtime() {
            let mut r: MutexGuard<'_, BufReader<File>> = self.reader.lock().unwrap();

            let mut new_buff = Vec::new();

            // todo - Iterate over the file and remove the entry
            // todo - later we need to find a better solution for this as its not preformat to iterate over the whole database
            // todo - just to delete some data. Maybe we can use a linked list or something else? But for now this will do.
            loop {
                match bincode::deserialize_from::<_, Entry<S>>(&mut r.get_mut()) {
                    Ok(Entry { key: entry_key, .. }) => {
                        if entry_key != key {
                            new_buff.append(&mut bincode::serialize(&entry_key)?);
                        } else {
                            // Skip this entry
                            continue;
                        }
                    }
                    Err(e) => {
                        if let bincode::ErrorKind::Io(io_err) = e.as_ref() {
                            if io_err.kind() == io::ErrorKind::UnexpectedEof {
                                // Reached the end of the serialized data
                                break;
                            } else {
                                return Err(e.into());
                            }
                        }
                    }
                }
            }

            // Drop the reader so we can write to the file
            drop(r);

            // Write the new buffer to the file and sync it
            let mut w: MutexGuard<'_, BufWriter<File>> = self.writer.lock().unwrap();
            w.seek(SeekFrom::Start(0))?; // Seek to the beginning of the file
            w.write_all(&new_buff)?;
            w.flush()?;
            w.get_ref().sync_all()?;
        }

        log::info!("[DELETE] Key deleted: {}", key);

        Ok(())
    }

    /// Gets the current ttl if it exists.
    /// Function will also try the default ttl if configured else it will return None.
    fn get_ttl(&self, ttl: Option<Duration>) -> anyhow::Result<Option<DateTime<Utc>>>
    {
        if let Some(ttl) = ttl {
            Ok(Some(Utc::now() + chrono::Duration::from_std(ttl)?))
        } else {
            if let Some(default_ttl) = self.config.default_ttl {
                Ok(Some(Utc::now() + chrono::Duration::from_std(default_ttl)?))
            } else {
                Ok(None)
            }
        }
    }

    /// Checks if we need to use disk operations, the default is disk.
    fn is_disk_runtime(&self) -> bool
    {
        if let Some(r) = &self.config.runtime {
            match r._type {
                RuntTimeType::Memory => false,
                RuntTimeType::Disk => true,
            }
        } else {
            true
        }
    }
}

#[cfg(test)]
mod tests
{
    use super::*;

    #[test]
    fn test_database_new()
    {
        let config = DatabaseConfiguration::default();
        let db = Database::<String>::new(&config).unwrap();

        assert_eq!(db.config.path, config.path);
    }
}
