use crate::types::BinaryKv;
use bincode::deserialize_from;
use log::LevelFilter;
use rayon::prelude::*;
use serde::de::DeserializeOwned;
use serde::Serialize;
use simple_logger::SimpleLogger;
use std::collections::HashMap;
use std::fmt::Debug;
use std::fs::{File, OpenOptions};
use std::hash::Hash;
use std::io::{self, Seek, SeekFrom, Write};
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

#[derive(Debug, Clone)]
pub struct Configuration {
    pub path: Option<PathBuf>,
    pub logs: bool,
    pub log_level: Option<LevelFilter>,
}

impl Default for Configuration {
    fn default() -> Self {
        Self {
            path: Some(PathBuf::from("db.qkv")),
            logs: false,
            log_level: Some(LevelFilter::Info),
        }
    }
}

/// The Schema client is a more optimized and faster version of the normal client.
///
/// It allows you to define a schema for your data, which will be used to serialize and deserialize your data.
/// The benefit is all operations are optimized for your data type, it also makes typings easier to work with.
/// Use this client when you want to work with data-modules that you have defined. The normal client is good
/// for storing generic data that could change frequently.
#[cfg(feature = "full")]
#[derive(Debug)]
pub struct QuickSchemaClient<T>
where
    T: Serialize + DeserializeOwned + Clone + Debug + Eq + PartialEq + Hash,
{
    pub file: Arc<Mutex<File>>,
    pub cache: Mutex<HashMap<String, BinaryKv<T>>>,
    pub position: u64,
    pub config: Configuration,
}

impl<T> QuickSchemaClient<T>
where
    T: Serialize + DeserializeOwned + Clone + Debug + Eq + PartialEq + Hash,
{
    pub fn new(config: Option<Configuration>) -> std::io::Result<Self> {
        let config = match config {
            Some(config) => config,
            None => Configuration::default(),
        };

        if config.clone().logs {
            let log_level = config.clone().log_level.unwrap();
            SimpleLogger::new()
                .with_colors(true)
                .with_threads(true)
                .with_level(log_level)
                .init()
                .unwrap();
        }

        let file = match OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .open(&config.clone().path.unwrap())
        {
            Ok(file) => file,
            Err(e) => {
                return Err(io::Error::new(
                    io::ErrorKind::Other,
                    format!("Error opening file: {:?}", e),
                ));
            }
        };

        log::info!("QuickSchemaClient Initialized!");

        Ok(Self {
            file: Arc::new(Mutex::new(file)),
            cache: Mutex::new(HashMap::new()),
            position: 0,
            config,
        })
    }

    pub fn get(&mut self, key: &str) -> std::io::Result<Option<T>> {
        log::info!("[GET] Searching for key: {}", key);

        // Check if the key is in the cache first
        let cache = self.cache.lock().unwrap();
        if let Some(entry) = cache.get(key) {
            log::debug!("[GET] Found cached key: {}", key);
            return Ok(Some(entry.value.clone()));
        }

        // If not in the cache, lock the file for reading
        let mut file = match self.file.lock() {
            Ok(file) => file,
            Err(e) => {
                return Err(io::Error::new(
                    io::ErrorKind::Other,
                    format!("Error locking file: {:?}", e),
                ));
            }
        };

        let mut reader = io::BufReader::new(&mut *file);

        reader.seek(SeekFrom::Start(self.position))?;

        // Read and deserialize entries until the end of the file is reached
        loop {
            match deserialize_from::<_, BinaryKv<T>>(&mut reader) {
                Ok(BinaryKv {
                    key: entry_key,
                    value,
                }) if key == entry_key => {
                    // Cache the deserialized entry
                    self.cache.lock().unwrap().insert(
                        key.to_string(),
                        BinaryKv::new(key.to_string(), value.clone()),
                    );
                    log::debug!("[GET] Caching uncached key: {}", key);

                    // Update the current position
                    self.position = reader.seek(SeekFrom::Current(0))?;

                    log::debug!("[GET] Found key: {}", key);
                    return Ok(Some(value));
                }
                Err(e) => {
                    if let bincode::ErrorKind::Io(io_err) = e.as_ref() {
                        if io_err.kind() == io::ErrorKind::UnexpectedEof {
                            // Reached the end of the serialized data
                            break;
                        }
                    }
                }
                _ => {}
            }
        }

        log::info!("[GET] Key not found: {}", key);

        // Key not found
        Ok(None)
    }

    pub fn set(&mut self, key: &str, value: T) -> std::io::Result<()> {
        log::info!("[SET] Setting key: {}", key);

        // First check if the data already exist, if so, update it not set it again.
        // This will stop memory alloc errors.
        if self.cache.lock().unwrap().get(key).is_some() {
            log::debug!("[SET] Key already exists, updating {} instead", key);
            return self.update(key, value);
        }

        let mut file = match self.file.lock() {
            Ok(file) => file,
            Err(e) => {
                return Err(io::Error::new(
                    io::ErrorKind::Other,
                    format!("Error locking file: {:?}", e),
                ));
            }
        };

        let mut writer = io::BufWriter::new(&mut *file);

        let data = BinaryKv::new(key.to_string(), value.clone());
        let serialized = match bincode::serialize(&data) {
            Ok(data) => data,
            Err(e) => {
                return Err(io::Error::new(
                    io::ErrorKind::Other,
                    format!("Error serializing data: {:?}", e),
                ));
            }
        };

        // Write the serialized data to the file
        writer.write_all(&serialized)?;
        writer.get_ref().sync_all()?;

        self.cache.lock().unwrap().insert(
            key.to_string(),
            BinaryKv::new(key.to_string(), value.clone()),
        );

        log::info!("[SET] Key set: {}", key);

        Ok(())
    }

    pub fn delete(&mut self, key: &str) -> std::io::Result<()> {
        log::info!("[DELETE] Deleting key: {}", key);

        // If the key is not in the cache, dont do anything as it doesn't exist on the file.
        if self.cache.lock().unwrap().remove(key).is_none() {
            return Ok(());
        }

        let mut file = match self.file.lock() {
            Ok(file) => file,
            Err(e) => {
                return Err(io::Error::new(
                    io::ErrorKind::Other,
                    format!("Error locking file: {:?}", e),
                ));
            }
        };

        let mut reader = io::BufReader::new(&mut *file);

        // Create a temporary buffer to store the updated data
        let mut updated_buffer = Vec::new();

        // Read and process entries
        loop {
            let current_position = reader.seek(SeekFrom::Current(0))?;

            match deserialize_from::<_, BinaryKv<T>>(&mut reader) {
                Ok(BinaryKv { key: entry_key, .. }) if key != entry_key => {
                    // Keep entries that don't match the key
                    updated_buffer.extend_from_slice(reader.buffer());

                    // Update the current position
                    self.position = reader.seek(SeekFrom::Start(current_position))?;
                }
                Ok(_) => {
                    // Skip entries that match the key
                    self.position = reader.seek(SeekFrom::Start(current_position))?;
                }
                Err(e) => {
                    if let bincode::ErrorKind::Io(io_err) = e.as_ref() {
                        if io_err.kind() == io::ErrorKind::UnexpectedEof {
                            // Reached the end of the serialized data
                            break;
                        }
                    }
                }
            }
        }

        // Close the file and open it in write mode for writing
        drop(reader); // Release the reader

        let mut writer = io::BufWriter::new(&mut *file);

        // Truncate the file and write the updated data back
        writer.get_mut().set_len(0)?;
        writer.seek(SeekFrom::Start(0))?;
        writer.write_all(&updated_buffer)?;
        writer.get_ref().sync_all()?;

        self.cache.lock().unwrap().remove(key);
        log::debug!("[DELETE] Cache deleted: {}", key);

        log::info!("[DELETE] Key deleted: {}", key);

        Ok(())
    }

    pub fn update(&mut self, key: &str, value: T) -> std::io::Result<()> {
        log::info!("[UPDATE] Updating key: {}", key);

        if self.cache.lock().unwrap().get(key).is_none() {
            log::debug!("[UPDATE] Key not found, attempting to set {} instead", key);
            return self.set(key, value);
        };

        let mut file = match self.file.lock() {
            Ok(file) => file,
            Err(e) => {
                return Err(io::Error::new(
                    io::ErrorKind::Other,
                    format!("Error locking file: {:?}", e),
                ));
            }
        };

        let mut reader = io::BufReader::new(&mut *file);

        reader.seek(SeekFrom::Start(self.position))?;

        let mut updated_entries = Vec::new();
        let mut updated = false;

        // Read and process entries
        loop {
            match deserialize_from::<_, BinaryKv<T>>(&mut reader) {
                Ok(entry) => {
                    if key == entry.key {
                        // Update the value associated with the key
                        let mut updated_entry = entry.clone();
                        updated_entry.value = value.clone();
                        updated_entries.push(updated_entry);
                        updated = true;
                    } else {
                        updated_entries.push(entry);
                    }
                }
                Err(e) => {
                    if let bincode::ErrorKind::Io(io_err) = e.as_ref() {
                        if io_err.kind() == io::ErrorKind::UnexpectedEof {
                            // Reached the end of the serialized data
                            break;
                        }
                    }
                }
            }
        }

        if !updated {
            log::warn!("[UPDATE] Key not found: {}. This should not trigger, if it did some cache may be invalid.", key);
            // Key not found
            return Err(io::Error::new(
                io::ErrorKind::Other,
                format!("Key not found: {}", key),
            ));
        }

        // Close the file and open it in write mode
        drop(reader); // Release the reader

        // Reopen the file in write mode for writing
        let mut writer = io::BufWriter::new(&mut *file);

        // Truncate the file and write the updated data back
        writer.get_mut().set_len(0)?;
        writer.seek(SeekFrom::Start(0))?;
        for entry in updated_entries.iter() {
            let serialized = match bincode::serialize(entry) {
                Ok(data) => data,
                Err(e) => {
                    return Err(io::Error::new(
                        io::ErrorKind::Other,
                        format!("Error serializing data: {:?}", e),
                    ));
                }
            };
            writer.write_all(&serialized)?;
        }

        writer.get_ref().sync_all()?;

        // Update the cache
        self.cache.lock().unwrap().insert(
            key.to_string(),
            BinaryKv::new(key.to_string(), value.clone()),
        );
        log::debug!("[UPDATE] Cache updated: {}", key);

        log::info!("[UPDATE] Key updated: {}", key);

        Ok(())
    }

    pub fn clear(&mut self) -> std::io::Result<()> {
        log::info!("[CLEAR] Clearing database");

        let mut file = match self.file.lock() {
            Ok(file) => file,
            Err(e) => {
                return Err(io::Error::new(
                    io::ErrorKind::Other,
                    format!("Error locking file: {:?}", e),
                ));
            }
        };

        let mut writer = io::BufWriter::new(&mut *file);

        writer.get_mut().set_len(0)?;
        writer.seek(SeekFrom::Start(0))?;
        writer.get_ref().sync_all()?;

        self.cache.lock().unwrap().clear();
        log::debug!("[CLEAR] Cache cleared");

        log::info!("[CLEAR] Database cleared");

        Ok(())
    }

    pub fn get_all(&mut self) -> std::io::Result<Vec<BinaryKv<T>>> {
        log::info!("[GET_ALL] Fetching all data in db cache...");

        let all_results = self
            .cache
            .lock()
            .unwrap()
            .iter()
            .map(|(_, entry)| entry.clone())
            .collect();

        log::info!("[GET_ALL] Fetched all data in db");

        Ok(all_results)
    }

    pub fn get_many(&mut self, keys: Vec<String>) -> std::io::Result<Vec<BinaryKv<T>>> {
        log::info!("[GET_MANY] Fetching many keys from db cache...");

        let mut results = Vec::new();

        for key in keys {
            if let Some(entry) = self.cache.lock().unwrap().get(&key) {
                results.push(entry.clone());
            }
        }

        log::info!("[GET_MANY] Fetched {} keys from db", results.len());

        Ok(results)
    }

    pub fn set_many(&mut self, values: Vec<BinaryKv<T>>) -> std::io::Result<()> {
        log::info!("[SET_MANY] Setting many keys in db...");

        // First check if the data already exist, if so, update it not set it again.
        // This will stop memory alloc errors.
        let mut to_update = Vec::new();

        for entry in values.iter() {
            if self.cache.lock().unwrap().get(&entry.key).is_some() {
                to_update.push(entry.clone());
            }
        }

        if !to_update.is_empty() {
            log::debug!(
                "[SET_MANY] Found {} keys that already exist, updating them instead of calling set",
                to_update.len()
            );
            self.update_many(to_update)?;
        }

        let mut file = match self.file.lock() {
            Ok(file) => file,
            Err(e) => {
                return Err(io::Error::new(
                    io::ErrorKind::Other,
                    format!("Error locking file: {:?}", e),
                ));
            }
        };

        let mut writer = io::BufWriter::new(&mut *file);
        let mut serialized = Vec::new();

        for entry in values.iter() {
            serialized.push(BinaryKv::new(entry.key.clone(), entry.value.clone()))
        }

        log::debug!("[SET_MANY] Serialized {} keys", serialized.len());

        let serialized = match bincode::serialize(&serialized) {
            Ok(data) => data,
            Err(e) => {
                return Err(io::Error::new(
                    io::ErrorKind::Other,
                    format!("Error serializing data: {:?}", e),
                ));
            }
        };

        log::debug!("[SET_MANY] Serialized {} keys", serialized.len());

        // Write the serialized data to the file
        writer.write_all(&serialized)?;
        writer.get_ref().sync_all()?;

        log::debug!("[SET_MANY] Wrote {} keys to file", serialized.len());

        for entry in values.iter() {
            self.cache.lock().unwrap().insert(
                entry.key.clone(),
                BinaryKv::new(entry.key.clone(), entry.value.clone()),
            );
        }

        log::info!("[SET_MANY] Set {} keys in db", values.len());

        Ok(())
    }

    pub fn delete_many(&mut self, keys: Vec<String>) -> std::io::Result<()> {
        log::info!("[DELETE_MANY] Deleting many keys from db...");

        if self.cache.lock().unwrap().is_empty() {
            log::debug!("[DELETE_MANY] Cache is empty, nothing to delete");
            return Ok(());
        }

        // First we check if any of the keys passed exist, before we search the file for them.
        let mut valid_keys = Vec::new();
        for key in keys {
            if self.cache.lock().unwrap().get(&key).is_some() {
                valid_keys.push(key)
            }
        }

        // Clone the valid_keys vector
        let vkc = valid_keys.clone();

        if valid_keys.is_empty() {
            log::debug!("[DELETE_MANY] No valid keys found, nothing to delete");
            return Ok(());
        }

        let mut file = match self.file.lock() {
            Ok(file) => file,
            Err(e) => {
                return Err(io::Error::new(
                    io::ErrorKind::Other,
                    format!("Error locking file: {:?}", e),
                ));
            }
        };

        let mut reader = io::BufReader::new(&mut *file);

        // Create a temporary buffer to store the updated data
        let mut updated_buffer = Vec::new();

        // Read and process entries
        loop {
            let current_position = reader.seek(SeekFrom::Current(0))?;

            match deserialize_from::<_, BinaryKv<T>>(&mut reader) {
                Ok(BinaryKv { key: entry_key, .. }) if valid_keys.contains(&entry_key) => {
                    // Keep entries that don't match the key
                    updated_buffer.extend_from_slice(reader.buffer());

                    // Update the current position
                    self.position = reader.seek(SeekFrom::Start(current_position))?;
                }
                Ok(_) => {
                    // Skip entries that match the key
                    self.position = reader.seek(SeekFrom::Start(current_position))?;
                }
                Err(e) => {
                    if let bincode::ErrorKind::Io(io_err) = e.as_ref() {
                        if io_err.kind() == io::ErrorKind::UnexpectedEof {
                            // Reached the end of the serialized data
                            break;
                        }
                    }
                }
            }
        }

        // Close the file and open it in write mode for writing
        drop(reader); // Release the reader

        let mut writer = io::BufWriter::new(&mut *file);

        // Truncate the file and write the updated data back
        writer.get_mut().set_len(0)?;
        writer.seek(SeekFrom::Start(0))?;
        writer.write_all(&updated_buffer)?;
        writer.get_ref().sync_all()?;

        for key in valid_keys {
            self.cache.lock().unwrap().remove(&key);
        }

        log::info!("[DELETE_MANY] Deleted {} keys from db", vkc.len());

        Ok(())
    }

    pub fn update_many(&mut self, values: Vec<BinaryKv<T>>) -> std::io::Result<()> {
        log::info!("[UPDATE_MANY] Updating many keys in db...");

        let mut to_set = Vec::new();

        for entry in values.iter() {
            if self.cache.lock().unwrap().get(&entry.key).is_none() {
                to_set.push(entry.clone());
            }
        }

        if !to_set.is_empty() {
            log::debug!("[UPDATE_MANY] Found {} keys that dont exist, setting them instead of calling update", to_set.len());
            return self.set_many(to_set);
        }

        let mut file = match self.file.lock() {
            Ok(file) => file,
            Err(e) => {
                return Err(io::Error::new(
                    io::ErrorKind::Other,
                    format!("Error locking file: {:?}", e),
                ));
            }
        };

        let mut reader = io::BufReader::new(&mut *file);

        reader.seek(SeekFrom::Start(self.position))?;

        let mut updated_entries = Vec::new();

        // Read and process entries
        loop {
            match deserialize_from::<_, BinaryKv<T>>(&mut reader) {
                Ok(entry) => {
                    if let Some(value) = values.iter().find(|v| v.key == entry.key) {
                        // Update the value associated with the key
                        let mut updated_entry = entry.clone();
                        updated_entry.value = value.value.clone();
                        updated_entries.push(updated_entry);
                    } else {
                        updated_entries.push(entry);
                    }
                }
                Err(e) => {
                    if let bincode::ErrorKind::Io(io_err) = e.as_ref() {
                        if io_err.kind() == io::ErrorKind::UnexpectedEof {
                            // Reached the end of the serialized data
                            break;
                        }
                    }
                }
            }
        }

        // Close the file and open it in write mode
        drop(reader); // Release the reader

        // Reopen the file in write mode for writing
        let mut writer = io::BufWriter::new(&mut *file);

        let mut serialized = Vec::new();

        for entry in updated_entries.iter() {
            serialized.push(BinaryKv::new(entry.key.clone(), entry.value.clone()))
        }

        let serialized = match bincode::serialize(&serialized) {
            Ok(data) => data,
            Err(e) => {
                return Err(io::Error::new(
                    io::ErrorKind::Other,
                    format!("Error serializing data: {:?}", e),
                ));
            }
        };

        log::debug!("[UPDATE_MANY] Serialized {} keys", serialized.len());

        // Truncate the file and write the updated data back
        writer.get_mut().set_len(0)?;
        writer.seek(SeekFrom::Start(0))?;
        writer.write_all(&serialized)?;
        writer.get_ref().sync_all()?;

        log::debug!("[UPDATE_MANY] Wrote {} keys to file", serialized.len());

        for entry in updated_entries.iter() {
            self.cache.lock().unwrap().insert(
                entry.key.clone(),
                BinaryKv::new(entry.key.clone(), entry.value.clone()),
            );
        }

        log::info!("[UPDATE_MANY] Updated {} keys in db", values.len());

        Ok(())
    }
}
