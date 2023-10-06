use std::collections::HashMap;
use std::fmt::Debug;
use std::fs::{File, OpenOptions};
use std::hash::Hash;
use std::io::{self, BufRead, Seek, SeekFrom, Write};
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use bincode::deserialize_from;
use log::LevelFilter;
use rayon::prelude::*;
use serde::de::DeserializeOwned;
use serde::Serialize;
use simple_logger::SimpleLogger;
use time::macros::format_description;

use crate::types::BinaryKv;

#[derive(Debug, Clone)]
pub struct QuickConfiguration
{
    pub path: Option<PathBuf>,
    pub logs: bool,
    pub log_level: Option<LevelFilter>,
}

impl QuickConfiguration
{
    pub fn new(path: Option<PathBuf>, logs: bool, log_level: Option<LevelFilter>) -> Self
    {
        Self { path, logs, log_level }
    }
}

impl Default for QuickConfiguration
{
    fn default() -> Self
    {
        Self {
            path: Some(PathBuf::from("db.qkv")),
            logs: false,
            log_level: Some(LevelFilter::Info),
        }
    }
}

/// The default and recommended client to use. It is optimized for a specific schema and has multi-threading enabled by default.
///
/// It allows you to define a schema for your data, which will be used to serialize and deserialize
/// your data. The benefit is all operations are optimized for your data type, it also makes typings
/// easier to work with. Use this client when you want to work with data-modules that you have
/// defined. The mini client is good for storing generic data that could change frequently.
///
/// # Example
/// ```rust
/// use std::path::PathBuf;
///
/// use quick_kv::prelude::*;
/// use serde::{Deserialize, Serialize};
///
/// #[derive(Serialize, Deserialize, Debug, PartialEq, Eq, Hash, Clone)]
/// struct User
/// {
///     name: String,
///     age: u8,
/// }
///
/// let config = QuickConfiguration::new(Some(PathBuf::from("db.qkv")), true, None);
///
/// let mut client = QuickClient::<User>::new(Some(config)).unwrap();
///
/// let user = User {
///     name: "John".to_string(),
///     age: 20,
/// };
///
/// client.set("user", user.clone()).unwrap();
///
/// let user_from_db = client.get("user").unwrap().unwrap();
///
/// assert_eq!(user, user_from_db);
/// ```
#[cfg(feature = "full")]
#[derive(Debug, Clone)]
pub struct QuickClient<T>
where
    T: Serialize + DeserializeOwned + Clone + Debug + Eq + PartialEq + Hash + Send + Sync,
{
    pub file: Arc<Mutex<File>>,
    pub cache: Arc<Mutex<HashMap<String, BinaryKv<T>>>>,
    pub config: QuickConfiguration,
}

impl<T> QuickClient<T>
where
    T: Serialize + DeserializeOwned + Clone + Debug + Eq + PartialEq + Hash + Send + Sync,
{
    pub fn new(config: Option<QuickConfiguration>) -> std::io::Result<Self>
    {
        let config = match config {
            Some(config) => config,
            None => QuickConfiguration::default(),
        };

        if config.clone().logs {
            let log_level = match config.clone().log_level {
                Some(log_level) => log_level,
                None => QuickConfiguration::default().log_level.unwrap(),
            };
            SimpleLogger::new()
                .with_colors(true)
                .with_threads(true)
                .with_level(log_level)
                .with_timestamp_format(format_description!("[year]-[month]-[day] [hour]:[minute]:[second]"))
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
                return Err(io::Error::new(io::ErrorKind::Other, format!("Error opening file: {:?}", e)));
            }
        };

        log::info!("QuickSchemaClient Initialized!");

        Ok(Self {
            file: Arc::new(Mutex::new(file)),
            cache: Arc::new(Mutex::new(HashMap::new())),
            config,
        })
    }

    pub fn get(&mut self, key: &str) -> std::io::Result<Option<T>>
    where
        T: Clone,
    {
        log::info!("[GET] Searching for key: {}", key);

        // Check if the key is in the cache first
        {
            let cache = self.cache.lock().unwrap();
            if let Some(entry) = cache.get(key) {
                log::debug!("[GET] Found cached key: {}", key);
                return Ok(Some(entry.value.clone()));
            }
        }

        // If not in the cache, lock the file for reading
        let mut file = match self.file.lock() {
            Ok(file) => file,
            Err(e) => {
                return Err(io::Error::new(io::ErrorKind::Other, format!("Error locking file: {:?}", e)));
            }
        };

        let mut reader = io::BufReader::new(&mut *file);

        // Set the position if the reader
        reader.seek(SeekFrom::Start(0))?;

        let key_clone = key.to_string();

        // Read and deserialize entries in parallel until the end of the file is reached
        let result = reader
            .lines()
            .par_bridge()
            .filter_map(|line| {
                if let Ok(line) = line {
                    let mut line_reader = io::Cursor::new(line);
                    match deserialize_from::<_, BinaryKv<T>>(&mut line_reader) {
                        Ok(BinaryKv { key: entry_key, value }) if key == entry_key => {
                            // Cache the deserialized entry
                            self.cache
                                .lock()
                                .unwrap()
                                .insert(key_clone.clone(), BinaryKv::new(key_clone.clone(), value.clone()));
                            log::debug!("[GET] Caching uncached key: {}", key_clone);

                            log::debug!("[GET] Found key: {}", key_clone);
                            Some(value)
                        }
                        Err(e) => {
                            if let bincode::ErrorKind::Io(io_err) = e.as_ref() {
                                if io_err.kind() == io::ErrorKind::UnexpectedEof {
                                    // Reached the end of the serialized data
                                    None
                                } else {
                                    None
                                }
                            } else {
                                None
                            }
                        }
                        _ => None,
                    }
                } else {
                    None
                }
            })
            .collect::<Vec<T>>();

        if result.is_empty() {
            log::debug!("[GET] Key not found: {}", key);
            return Ok(None);
        }

        log::info!("[GET] Key found: {}", key);

        Ok(Some(result[0].clone()))
    }

    pub fn set(&mut self, key: &str, value: T) -> std::io::Result<()>
    {
        log::info!("[SET] Setting key: {}", key);

        // First check if the data already exist, if so, update it not set it again.
        // This will stop memory alloc errors.
        {
            if self.cache.lock().unwrap().get(key).is_some() {
                log::debug!("[SET] Key already exists, updating {} instead", key);
                return self.update(key, value);
            }
        }

        let data = BinaryKv::new(key.to_string(), value.clone());
        // Serialize the data in parallel
        rayon::scope(|s| {
            let data = &data; // Immutable reference to data

            s.spawn(move |_| {
                let mut file = self.file.lock().unwrap();
                let mut writer = io::BufWriter::new(&mut *file);
                let serialized = bincode::serialize(data).expect("Error serializing data");
                writer.write_all(&serialized).expect("Error writing data to file");
                writer.get_ref().sync_all().expect("Error syncing file");

                self.cache
                    .lock()
                    .unwrap()
                    .insert(key.to_string(), BinaryKv::new(key.to_string(), value.clone()));

                log::info!("[SET] Key set: {}", key);
            })
        });

        Ok(())
    }

    pub fn delete(&mut self, key: &str) -> std::io::Result<()>
    {
        log::info!("[DELETE] Deleting key: {}", key);

        // If the key is not in the cache, dont do anything as it doesn't exist on the file.
        {
            if self.cache.lock().unwrap().remove(key).is_none() {
                return Ok(());
            }
        }

        rayon::scope(|s| {
            s.spawn(move |_| {
                let mut file = self.file.lock().unwrap();
                let mut reader = io::BufReader::new(&mut *file);

                // Create a temporary buffer to store the updated data
                let mut updated_buffer = Vec::new();

                // Read and process entries
                loop {
                    match deserialize_from::<_, BinaryKv<T>>(&mut reader) {
                        Ok(BinaryKv { key: entry_key, .. }) if key != entry_key => {
                            // Keep entries that don't match the key
                            updated_buffer.extend_from_slice(reader.buffer());
                        }
                        Ok(_) => {}
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
                writer.get_mut().set_len(0).expect("Error truncating file");
                writer.seek(SeekFrom::Start(0)).expect("Error seeking file");
                writer.write_all(&updated_buffer).expect("Error writing data to file");
                writer.get_ref().sync_all().expect("Error syncing file");

                self.cache.lock().unwrap().remove(key);
                log::debug!("[DELETE] Cache deleted: {}", key);

                log::info!("[DELETE] Key deleted: {}", key);
            })
        });

        Ok(())
    }

    pub fn update(&mut self, key: &str, value: T) -> std::io::Result<()>
    {
        log::info!("[UPDATE] Updating key: {}", key);

        {
            if self.cache.lock().unwrap().get(key).is_none() {
                log::debug!("[UPDATE] Key not found, attempting to set {} instead", key);
                return self.set(key, value);
            };
        }

        let self_clone = self.clone();

        rayon::scope(|s| {
            s.spawn(move |_| {
                let mut file = self_clone.file.lock().unwrap();
                let mut reader = io::BufReader::new(&mut *file);

                reader.seek(SeekFrom::Start(0)).expect("Error seeking file");

                let mut updated_entries = Vec::new();
                let mut updated = false;

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

                if updated {
                    // Close the file and open it in write mode
                    drop(reader); // Release the reader

                    // Reopen the file in write mode for writing
                    let mut writer = io::BufWriter::new(&mut *file);

                    // Truncate the file and write the updated data back
                    writer.get_mut().set_len(0).expect("Error truncating file");
                    writer.seek(SeekFrom::Start(0)).expect("Error seeking file");

                    for entry in updated_entries.iter() {
                        let serialized = bincode::serialize(entry).expect("Error serializing data");
                        writer.write_all(&serialized).expect("Error writing data to file");
                    }

                    writer.get_ref().sync_all().expect("Error syncing file");

                    self_clone
                        .cache
                        .lock()
                        .unwrap()
                        .insert(key.to_string(), BinaryKv::new(key.to_string(), value.clone()));

                    log::debug!("[UPDATE] Cache updated: {}", key);

                    log::info!("[UPDATE] Key updated: {}", key);
                } else {
                    log::warn!(
                        "[UPDATE] Key not found: {}. This should not trigger, if it did some cache may be invalid.",
                        key
                    );
                }
            })
        });

        Ok(())
    }

    pub fn clear(&mut self) -> std::io::Result<()>
    {
        log::info!("[CLEAR] Clearing database");

        let mut file = match self.file.lock() {
            Ok(file) => file,
            Err(e) => {
                return Err(io::Error::new(io::ErrorKind::Other, format!("Error locking file: {:?}", e)));
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

    pub fn get_all(&mut self) -> std::io::Result<Vec<BinaryKv<T>>>
    {
        log::info!("[GET_ALL] Fetching all data in db cache...");

        let cache = &self.cache.lock().unwrap();

        let all_results: Vec<BinaryKv<T>> = cache
            .par_iter() // Parallelize the iteration over key-value pairs
            .map(|(_, entry)| entry.clone()) // Clone each entry in parallel
            .collect();

        log::info!("[GET_ALL] Fetched all data in db");

        Ok(all_results)
    }

    pub fn get_many(&mut self, keys: Vec<String>) -> std::io::Result<Vec<BinaryKv<T>>>
    {
        log::info!("[GET_MANY] Fetching many keys from db cache...");

        let cache_guard = self.cache.lock().unwrap();

        let results: Vec<BinaryKv<T>> = keys
            .par_iter() // Parallelize the iteration over keys
            .filter_map(|key| cache_guard.get(key).cloned()) // Filter and clone entries in parallel
            .collect();

        log::info!("[GET_MANY] Fetched {} keys from db", results.len());

        Ok(results)
    }

    pub fn set_many(&mut self, values: Vec<BinaryKv<T>>) -> std::io::Result<()>
    {
        log::info!("[SET_MANY] Setting many keys in db...");

        // First check if the data already exist, if so, update it not set it again.
        // This will stop memory alloc errors.
        let mut to_update = Vec::new();

        {
            let cache_guard = self.cache.lock().unwrap();

            for entry in values.iter() {
                if cache_guard.get(&entry.key).is_some() {
                    to_update.push(entry.clone());
                }
            }
        }

        if !to_update.is_empty() {
            log::debug!(
                "[SET_MANY] Found {} keys that already exist, updating them instead of calling set",
                to_update.len()
            );
            self.update_many(to_update)?;
        }

        rayon::scope(|s| {
            s.spawn(move |_| {
                let mut file = self.file.lock().unwrap();

                let mut reader = io::BufReader::new(&mut *file);
                reader.seek(SeekFrom::Start(0)).expect("Error seeking file");

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

                let serialized = bincode::serialize(&serialized).expect("Error serializing data");

                log::debug!("[SET_MANY] Serialized {} keys", serialized.len());

                // Truncate the file and write the updated data back
                writer.get_mut().set_len(0).expect("Error truncating file");
                writer.seek(SeekFrom::Start(0)).expect("Error seeking file");
                writer.write_all(&serialized).expect("Error writing data to file");
                writer.get_ref().sync_all().expect("Error syncing file");

                log::debug!("[SET_MANY] Wrote {} keys to file", serialized.len());

                {
                    let mut cache_guard = self.cache.lock().unwrap();

                    for entry in values.iter() {
                        cache_guard.insert(entry.key.clone(), BinaryKv::new(entry.key.clone(), entry.value.clone()));
                    }
                }

                log::info!("[SET_MANY] Set {} keys in db", values.len());
            })
        });

        Ok(())
    }

    pub fn delete_many(&mut self, keys: Vec<String>) -> std::io::Result<()>
    {
        log::info!("[DELETE_MANY] Deleting many keys from db...");

        {
            if self.cache.lock().unwrap().is_empty() {
                log::debug!("[DELETE_MANY] Cache is empty, nothing to delete");
                return Ok(());
            }
        }

        // First we check if any of the keys passed exist, before we search the file for them.
        let mut valid_keys = Vec::new();
        {
            let cache_guard = self.cache.lock().unwrap();

            for key in keys {
                if cache_guard.get(&key).is_some() {
                    valid_keys.push(key);
                }
            }
        }

        // Clone the valid_keys vector
        let vkc = valid_keys.clone();

        if valid_keys.is_empty() {
            log::debug!("[DELETE_MANY] No valid keys found, nothing to delete");
            return Ok(());
        }

        rayon::scope(|s| {
            s.spawn(move |_| {
                let mut file = self.file.lock().unwrap();
                let mut reader = io::BufReader::new(&mut *file);

                // Create a temporary buffer to store the updated data
                let mut updated_buffer = Vec::new();

                // Read and process entries
                loop {
                    match deserialize_from::<_, BinaryKv<T>>(&mut reader) {
                        Ok(BinaryKv { key: entry_key, .. }) if valid_keys.contains(&entry_key) => {
                            // Keep entries that don't match the key
                            updated_buffer.extend_from_slice(reader.buffer());
                        }
                        Ok(_) => {
                            // Skip entries that match the key
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
                writer.get_mut().set_len(0).expect("Error truncating file");
                writer.seek(SeekFrom::Start(0)).expect("Error seeking file");
                writer.write_all(&updated_buffer).expect("Error writing data to file");
                writer.get_ref().sync_all().expect("Error syncing file");

                let mut cache_guard = self.cache.lock().unwrap();

                for key in valid_keys {
                    cache_guard.remove(&key);
                }

                log::info!("[DELETE_MANY] Deleted {} keys from db", vkc.len());
            })
        });

        Ok(())
    }

    pub fn update_many(&mut self, values: Vec<BinaryKv<T>>) -> std::io::Result<()>
    {
        log::info!("[UPDATE_MANY] Updating many keys in db...");

        let mut to_set = Vec::new();

        {
            let cache_guard = self.cache.lock().unwrap();

            for entry in values.iter() {
                if cache_guard.get(&entry.key).is_none() {
                    to_set.push(entry.clone());
                }
            }
        }

        if !to_set.is_empty() {
            log::debug!(
                "[UPDATE_MANY] Found {} keys that dont exist, setting them instead of calling update",
                to_set.len()
            );
            return self.set_many(to_set);
        }

        rayon::scope(|s| {
            s.spawn(move |_| {
                let mut file = self.file.lock().unwrap();
                let mut reader = io::BufReader::new(&mut *file);
                reader.seek(SeekFrom::Start(0)).expect("Error seeking file");

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

                let serialized = bincode::serialize(&serialized).expect("Error serializing data");

                log::debug!("[UPDATE_MANY] Serialized {} keys", serialized.len());

                // Truncate the file and write the updated data back
                writer.get_mut().set_len(0).expect("Error truncating file");
                writer.seek(SeekFrom::Start(0)).expect("Error seeking file");
                writer.write_all(&serialized).expect("Error writing data to file");
                writer.get_ref().sync_all().expect("Error syncing file");

                log::debug!("[UPDATE_MANY] Wrote {} keys to file", serialized.len());

                let mut cache_guard = self.cache.lock().unwrap();

                for entry in updated_entries.iter() {
                    cache_guard.insert(entry.key.clone(), BinaryKv::new(entry.key.clone(), entry.value.clone()));
                }

                log::info!("[UPDATE_MANY] Updated {} keys in db", values.len());
            })
        });

        Ok(())
    }
}
