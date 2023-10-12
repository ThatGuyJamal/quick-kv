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
use crate::db::entry::Entry;
use crate::db::state::State;
use crate::types::HashMap;

pub(crate) mod batcher;
pub(crate) mod config;
pub(super) mod entry;
pub(super) mod runtime;
pub(super) mod state;

/// A signal sent to the background task.
#[derive(Debug)]
pub(super) enum TTLSignal
{
    Check,
    Exit,
}

/// The database consumed by clients.
///
/// Controls the state of the data-store and the background task.
#[derive(Debug)]
pub(crate) struct Database<T>
where
    T: Serialize + DeserializeOwned + Debug + Eq + PartialEq + Hash + Send + Sync + Clone + 'static,
{
    pub(super) state: Arc<Mutex<State<T>>>,
    pub(super) config: DatabaseConfiguration,
    // pub(super) ttl_manager: mpsc::Sender<TTLSignal>,
    pub(super) writer: Arc<Mutex<BufWriter<File>>>,
    pub(super) reader: Arc<Mutex<BufReader<File>>>,
}

impl<T> Database<T>
where
    T: Serialize + DeserializeOwned + Debug + Eq + PartialEq + Hash + Send + Sync + Clone + 'static,
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

        log::info!("[Bootstrap] Building Database State");

        let file = OpenOptions::new()
            .read(true)
            .write(true)
            .append(true)
            .create(true)
            .open(config.path.as_ref().unwrap())?;

        // Create two clones of the file handle, one for reading and one for writing.
        let file_clone = file.try_clone()?;
        let file_clone2 = file.try_clone()?;

        // let (sender, receiver) = mpsc::channel::<TTLSignal>();

        let output = Self {
            state: Arc::new(Mutex::new(State::new())),
            config,
            writer: Arc::new(Mutex::new(BufWriter::new(file_clone))),
            reader: Arc::new(Mutex::new(BufReader::new(file_clone2))),
        };

        log::info!("[Bootstrap] QuickSchemaClient Initialized!");

        Ok(output)
    }

    pub(crate) fn get(&mut self, key: String) -> anyhow::Result<Option<T>>
    {
        log::debug!("[GET] Searching for key: {}", key);

        // self.ttl_manager.send(TTLSignal::Check)?;

        let state = self.state.lock().unwrap();

        if let Some(entry) = state.entries.get(&key) {
            log::debug!("[GET] Found key: {}", key);
            return Ok(Some(entry.data.clone()));
        }

        Ok(None)

        // Maybe we will check file, if no cache is found. Although for now this should
        // Never happen so we will just return None if nothing is found.
    }

    pub(crate) fn set(&mut self, key: &str, value: T, ttl: Option<Duration>) -> anyhow::Result<()>
    {
        log::info!("[SET] Attempting set: {}", key);

        // First check if the data already exists; if so, update it instead
        let mut state = self.state.lock().unwrap();

        let expires_at: Option<DateTime<Utc>> = self.get_ttl(ttl)?;

        // Build the entry
        let entry = Entry::new(key.to_string(), value, expires_at);

        // Set the entry in the state
        state.entries.insert(key.to_string(), entry.clone());

        if let Some(expires_at) = entry.expires_at {
            state.expirations.insert((expires_at, key.to_string()));
        }

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

    pub(crate) fn update(&mut self, key: &str, value: T, ttl: Option<Duration>, upsert: Option<bool>) -> anyhow::Result<()>
    {
        log::info!("[UPDATE] Attempting {} update...", key);

        let mut state = self.state.lock().unwrap();

        if !state.entries.contains_key(key) {
            log::debug!("[UPDATE] Key not found: {}", key);
            return Ok(());
        }

        if let Some(u) = upsert {
            if !u {
                log::debug!("[UPDATE] Upsert not enabled, skipping update");
                return Ok(());
            }
        }

        let entry: Entry<T> = Entry::new(key.to_string(), value.clone(), None);

        state.entries.insert(key.to_string(), entry.clone());

        if let Some(expires_at) = entry.expires_at {
            state.expirations.insert((expires_at, key.to_string()));
        }

        if self.is_disk_runtime() {
            let mut r = self.reader.lock().unwrap();

            r.seek(SeekFrom::Start(0))?;

            let mut updated_bytes = Vec::new();

            loop {
                match bincode::deserialize_from::<_, Entry<T>>(&mut r.get_mut()) {
                    Ok(entry) => {
                        if key == entry.key {
                            // Update the value associated with the key
                            updated_bytes.push(Entry::new(key.to_string(), value.clone(), self.get_ttl(ttl)?));
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
                match bincode::deserialize_from::<_, Entry<T>>(&mut r.get_mut()) {
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

    pub(crate) fn purge(&mut self) -> anyhow::Result<()>
    {
        log::info!("[PURGE] Purging database");

        let mut state = self.state.lock().unwrap();

        state.entries.clear();
        state.expirations.clear();

        if self.is_disk_runtime() {
            let mut w: MutexGuard<'_, BufWriter<File>> = self.writer.lock().unwrap();

            w.seek(SeekFrom::Start(0))?; // Seek to the beginning of the file
            w.write_all(&[])?;
            w.flush()?;
            w.get_ref().sync_all()?;
        }

        log::info!("[PURGE] Database purged");

        Ok(())
    }

    /// Gets the current ttl if it exists.
    /// Function will also try the default ttl if configured else it will return None.
    fn get_ttl(&self, ttl: Option<Duration>) -> anyhow::Result<Option<DateTime<Utc>>>
    {
        if let Some(ttl) = ttl {
            Ok(Some(Utc::now() + chrono::Duration::from_std(ttl)?))
        } else if let Some(default_ttl) = self.config.default_ttl {
            Ok(Some(Utc::now() + chrono::Duration::from_std(default_ttl)?))
        } else {
            Ok(None)
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
    use anyhow::Result;
    use tempfile::tempdir;

    use super::*;

    #[test]
    fn test_database_new() -> Result<()>
    {
        let tmp_dir = tempdir().expect("Failed to create tempdir");
        let tmp_file = tmp_dir.path().join("test.qkv").to_str().unwrap().to_string();

        let config = DatabaseConfiguration::new(Some(tmp_file), None, None, None, None)?;
        let db = Database::<String>::new(config.clone())?;

        assert_eq!(db.config.path, config.path);

        Ok(())
    }

    #[test]
    fn test_database_get_set() -> Result<()>
    {
        let tmp_dir = tempdir().expect("Failed to create tempdir");
        let tmp_file = tmp_dir.path().join("test.qkv").to_str().unwrap().to_string();

        let config = DatabaseConfiguration::new(Some(tmp_file), None, None, None, None)?;
        let mut db = Database::<String>::new(config)?;

        db.set("test", "test".to_string(), None)?;

        assert_eq!(db.get("test".to_string()).unwrap().unwrap(), "test".to_string());

        Ok(())
    }

    #[test]
    fn test_database_update() -> Result<()>
    {
        let tmp_dir = tempdir().expect("Failed to create tempdir");
        let tmp_file = tmp_dir.path().join("test.qkv").to_str().unwrap().to_string();

        let config = DatabaseConfiguration::new(Some(tmp_file), None, None, None, None)?;

        let mut db = Database::<String>::new(config)?;

        db.set("test", "test".to_string(), None)?;

        let result = db.get("test".to_string())?.unwrap();

        db.update("test", "test2".to_string(), None, None)?;

        let result = db.get("test".to_string())?.unwrap();

        assert_eq!(result, "test2".to_string());

        Ok(())
    }

    #[test]
    fn test_database_delete() -> Result<()>
    {
        let tmp_dir = tempdir().expect("Failed to create tempdir");
        let tmp_file = tmp_dir.path().join("test.qkv").to_str().unwrap().to_string();

        let config = DatabaseConfiguration::new(Some(tmp_file), None, None, None, None)?;

        let mut db = Database::<String>::new(config)?;

        db.set("test", "test".to_string(), None)?;

        let result = db.get("test".to_string())?.unwrap();

        db.delete("test")?;

        let result = db.get("test".to_string())?;

        assert_eq!(result, None);

        Ok(())
    }
}
