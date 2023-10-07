use std::fmt::Debug;
use std::fs::{File, OpenOptions};
use std::hash::Hash;
use std::io::{self, BufRead, Seek, SeekFrom, Write};
use std::path::Path;
use std::sync::{Arc, Mutex};

use bincode::deserialize_from;
use hashbrown::HashMap;
use log::LevelFilter;
use rayon::prelude::*;
use serde::de::DeserializeOwned;
use serde::Serialize;
use simple_logger::SimpleLogger;
use time::macros::format_description;

use crate::types::binarykv::BinaryKv;
use crate::utils::validate_database_file_path;

/// Configurations for the client
#[derive(Debug, Clone)]
pub struct QuickConfiguration<'a>
{
    pub path: Option<&'a str>,
    pub logs: bool,
    pub log_level: Option<LevelFilter>,
}

impl<'a> QuickConfiguration<'a>
{
    pub fn new(path: Option<&'a str>, logs: bool, log_level: Option<LevelFilter>) -> Self
    {
        Self { path, logs, log_level }
    }
}

impl Default for QuickConfiguration<'_>
{
    fn default() -> Self
    {
        Self {
            path: None,
            logs: false,
            log_level: None,
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
/// let config = QuickConfiguration::new(Some("db.qkv"), true, None);
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
pub struct QuickClient<'a, T>
where
    T: Serialize + DeserializeOwned + Clone + Debug + Eq + PartialEq + Hash + Send + Sync,
{
    pub file: Arc<Mutex<File>>,
    pub cache: Arc<Mutex<HashMap<String, BinaryKv<T>>>>,
    pub config: QuickConfiguration<'a>,
}

impl<'a, T> QuickClient<'a, T>
where
    T: Serialize + DeserializeOwned + Clone + Debug + Eq + PartialEq + Hash + Send + Sync,
{
    /// Creates a new instance of the client
    ///
    /// `config` is an optional configuration struct that allows you to configure the client.
    pub fn new(config: Option<QuickConfiguration<'a>>) -> std::io::Result<Self>
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

        let path = validate_database_file_path(&config.clone().path.unwrap_or("db.qkv"));

        // Extract the directory part from the path
        let dir_path = Path::new(&path).parent().unwrap_or_else(|| Path::new(""));

        // Create the parent directories if they don't exist
        if !dir_path.exists() {
            std::fs::create_dir_all(dir_path)?;
        }

        let file = match OpenOptions::new().read(true).write(true).create(true).open(path) {
            Ok(file) => file,
            Err(e) => {
                return Err(io::Error::new(io::ErrorKind::Other, format!("Error opening file: {:?}", e)));
            }
        };

        log::info!("QuickSchemaClient Initialized!");

        Ok(Self {
            file: Arc::new(Mutex::new(file)),
            cache: Arc::new(Mutex::new(HashMap::new())),
            config: config.clone(),
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

        // First check if the data already exists; if so, update it instead
        {
            if self.cache.lock().unwrap().get(key).is_some() {
                log::debug!("[SET] Key already exists, updating {} instead", key);
                return self.update(key, value);
            }
        }

        let mut file = match self.file.lock() {
            Ok(file) => file,
            Err(e) => {
                return Err(io::Error::new(io::ErrorKind::Other, format!("Error locking file: {:?}", e)));
            }
        };

        let mut writer = io::BufWriter::new(&mut *file);

        let data = BinaryKv::new(key.to_string(), value.clone());
        // Serialize the data in parallel and wait for it to complete
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
        writer.flush()?;
        writer.get_ref().sync_all()?;

        self.cache
            .lock()
            .unwrap()
            .insert(key.to_string(), BinaryKv::new(key.to_string(), value.clone()));

        log::info!("[SET] Key set: {}", key);

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

        let mut file = match self.file.lock() {
            Ok(file) => file,
            Err(e) => {
                return Err(io::Error::new(io::ErrorKind::Other, format!("Error locking file: {:?}", e)));
            }
        };

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
        writer.seek(SeekFrom::Start(0))?;
        writer.write_all(&updated_buffer)?;
        writer.flush()?;
        writer.get_ref().sync_all()?;

        self.cache.lock().unwrap().remove(key);
        log::debug!("[DELETE] Cache deleted: {}", key);

        log::info!("[DELETE] Key deleted: {}", key);

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

        let mut file = match self.file.lock() {
            Ok(file) => file,
            Err(e) => {
                return Err(io::Error::new(io::ErrorKind::Other, format!("Error locking file: {:?}", e)));
            }
        };

        let mut reader = io::BufReader::new(&mut *file);

        reader.seek(SeekFrom::Start(0))?;

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
            log::warn!(
                "[UPDATE] Key not found: {}. This should not trigger, if it did some cache may be invalid.",
                key
            );
            // Key not found
            return Err(io::Error::new(io::ErrorKind::Other, format!("Key not found: {}", key)));
        }

        // Close the file and open it in write mode
        drop(reader); // Release the reader

        // Reopen the file in write mode for writing
        let mut writer = io::BufWriter::new(&mut *file);

        // Truncate the file and write the updated data back
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

        writer.flush()?;
        writer.get_ref().sync_all()?;

        // Update the cache
        self.cache
            .lock()
            .unwrap()
            .insert(key.to_string(), BinaryKv::new(key.to_string(), value.clone()));
        log::debug!("[UPDATE] Cache updated: {}", key);

        log::info!("[UPDATE] Key updated: {}", key);

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
        writer.flush()?;
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

        let mut file = match self.file.lock() {
            Ok(file) => file,
            Err(e) => {
                return Err(io::Error::new(io::ErrorKind::Other, format!("Error locking file: {:?}", e)));
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

        // Write the serialized data to the file
        writer.write_all(&serialized)?;
        writer.flush()?;
        writer.get_ref().sync_all()?;

        log::debug!("[SET_MANY] Wrote {} keys to file", serialized.len());

        {
            let mut cache_guard = self.cache.lock().unwrap();

            for entry in values.iter() {
                cache_guard.insert(entry.key.clone(), BinaryKv::new(entry.key.clone(), entry.value.clone()));
            }
        }

        log::info!("[SET_MANY] Set {} keys in db", values.len());

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

        let mut file = match self.file.lock() {
            Ok(file) => file,
            Err(e) => {
                return Err(io::Error::new(io::ErrorKind::Other, format!("Error locking file: {:?}", e)));
            }
        };

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
        writer.seek(SeekFrom::Start(0))?;
        writer.write_all(&updated_buffer)?;
        writer.flush()?;
        writer.get_ref().sync_all()?;

        for key in valid_keys {
            self.cache.lock().unwrap().remove(&key);
        }

        log::info!("[DELETE_MANY] Deleted {} keys from db", vkc.len());

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

        let mut file = match self.file.lock() {
            Ok(file) => file,
            Err(e) => {
                return Err(io::Error::new(io::ErrorKind::Other, format!("Error locking file: {:?}", e)));
            }
        };

        let mut reader = io::BufReader::new(&mut *file);

        reader.seek(SeekFrom::Start(0))?;

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
        writer.seek(SeekFrom::Start(0))?;
        writer.write_all(&serialized)?;
        writer.flush()?;
        writer.get_ref().sync_all()?;

        log::debug!("[UPDATE_MANY] Wrote {} keys to file", serialized.len());

        for entry in updated_entries.iter() {
            self.cache
                .lock()
                .unwrap()
                .insert(entry.key.clone(), BinaryKv::new(entry.key.clone(), entry.value.clone()));
        }

        log::info!("[UPDATE_MANY] Updated {} keys in db", values.len());

        Ok(())
    }
}

#[cfg(feature = "full")]
#[cfg(test)]
mod feature_tests
{
    use tempfile::tempdir;

    use crate::prelude::*;

    #[test]
    fn test_client_new() -> std::io::Result<()>
    {
        let tmp_dir = tempdir().expect("Failed to create tempdir");
        let tmp_file = tmp_dir.path().join("test.qkv");

        match QuickClient::<String>::new(Some(QuickConfiguration {
            path: Some(tmp_file.to_str().unwrap()),
            ..Default::default()
        })) {
            Ok(_) => Ok(()),
            Err(e) => Err(std::io::Error::new(
                std::io::ErrorKind::Other,
                format!("Failed to create QuickClient: {}", e),
            )),
        }
    }

    #[test]
    fn test_get_and_set() -> std::io::Result<()>
    {
        let tmp_dir = tempdir().expect("Failed to create tempdir");
        let tmp_file = tmp_dir.path().join("test.qkv");

        let mut client = QuickClient::<String>::new(Some(QuickConfiguration {
            path: Some(tmp_file.to_str().unwrap()),
            ..Default::default()
        }))?;

        client.set("hello", String::from("Hello World!"))?;

        let result = client.get("hello")?;

        assert_eq!(result, Some(String::from("Hello World!")));

        Ok(())
    }

    #[test]
    fn test_clear() -> std::io::Result<()>
    {
        let tmp_dir = tempdir().expect("Failed to create tempdir");
        let tmp_file = tmp_dir.path().join("test.qkv");

        let mut client = QuickClient::<i32>::new(Some(QuickConfiguration {
            path: Some(tmp_file.to_str().unwrap()),
            ..Default::default()
        }))?;

        // Add some data to the cache
        client.set("key1", 42)?;
        client.set("key2", 77)?;

        // Call clear to remove data from cache and file
        client.clear()?;

        // Check if cache is empty
        let cache = client.cache.lock().unwrap();
        assert!(cache.is_empty());

        Ok(())
    }

    #[test]
    fn test_get_all() -> std::io::Result<()>
    {
        let tmp_dir = tempdir().expect("Failed to create tempdir");
        let tmp_file = tmp_dir.path().join("test.qkv");

        let mut client = QuickClient::<i32>::new(Some(QuickConfiguration {
            path: Some(tmp_file.to_str().unwrap()),
            ..Default::default()
        }))?;

        // Add some data to the cache
        client.set("key1", 42)?;
        client.set("key2", 77)?;

        // Get all data from the cache
        let all_data = client.get_all()?;

        // Check if all data is retrieved correctly
        assert_eq!(all_data.len(), 2);
        assert!(all_data.contains(&BinaryKv::new("key1".to_string(), 42)));
        assert!(all_data.contains(&BinaryKv::new("key2".to_string(), 77)));

        Ok(())
    }

    #[test]
    fn test_get_many() -> std::io::Result<()>
    {
        let tmp_dir = tempdir().expect("Failed to create tempdir");
        let tmp_file = tmp_dir.path().join("test.qkv");

        let mut client = QuickClient::<i32>::new(Some(QuickConfiguration {
            path: Some(tmp_file.to_str().unwrap()),
            ..Default::default()
        }))?;

        // Add some data to the cache
        client.set("key1", 42)?;
        client.set("key2", 77)?;

        // Get specific keys from the cache
        let keys_to_get = vec!["key1".to_string(), "key2".to_string()];
        let values = client.get_many(keys_to_get)?;

        // Check if values are retrieved correctly
        assert_eq!(values.len(), 2);
        assert_eq!(values[0], BinaryKv::new("key1".to_string(), 42));
        assert_eq!(values[1], BinaryKv::new("key2".to_string(), 77));

        Ok(())
    }

    #[test]
    fn test_set_many() -> std::io::Result<()>
    {
        let tmp_dir = tempdir().expect("Failed to create tempdir");
        let tmp_file = tmp_dir.path().join("test.qkv");

        let mut client = QuickClient::<i32>::new(Some(QuickConfiguration {
            path: Some(tmp_file.to_str().unwrap()),
            ..Default::default()
        }))?;

        // Set multiple values
        let values = vec![BinaryKv::new("key1".to_string(), 42), BinaryKv::new("key2".to_string(), 77)];
        client.set_many(values)?;

        // Check if values are set correctly in the cache
        let cache = client.cache.lock().unwrap();
        assert_eq!(cache.len(), 2);
        assert_eq!(cache.get("key1"), Some(&BinaryKv::new("key1".to_string(), 42)));
        assert_eq!(cache.get("key2"), Some(&BinaryKv::new("key2".to_string(), 77)));

        Ok(())
    }

    #[test]
    fn test_delete_many() -> std::io::Result<()>
    {
        let tmp_dir = tempdir().expect("Failed to create tempdir");
        let tmp_file = tmp_dir.path().join("test.qkv");

        let mut client = QuickClient::<i32>::new(Some(QuickConfiguration {
            path: Some(tmp_file.to_str().unwrap()),
            ..Default::default()
        }))?;

        // Add some data to the cache
        client.set("key1", 42)?;
        client.set("key2", 77)?;

        // Delete specific keys from the cache and file
        let keys_to_delete = vec!["key1".to_string(), "key2".to_string()];
        client.delete_many(keys_to_delete)?;

        // Check if keys are deleted from the cache
        let cache = client.cache.lock().unwrap();
        assert!(cache.is_empty());

        Ok(())
    }

    #[test]
    fn test_update_many() -> std::io::Result<()>
    {
        let tmp_dir = tempdir().expect("Failed to create tempdir");
        let tmp_file = tmp_dir.path().join("test.qkv");

        let mut client = QuickClient::<i32>::new(Some(QuickConfiguration {
            path: Some(tmp_file.to_str().unwrap()),
            ..Default::default()
        }))?;

        client.set("key1", 42)?;
        client.set("key2", 77)?;

        let keys_to_update = vec![BinaryKv::new("key1".to_string(), 22), BinaryKv::new("key2".to_string(), 454)];

        client.update_many(keys_to_update)?;

        let cache = client.cache.lock().unwrap();
        assert_eq!(cache.len(), 2);
        assert_eq!(cache.get("key1"), Some(&BinaryKv::new("key1".to_string(), 22)));
        assert_eq!(cache.get("key2"), Some(&BinaryKv::new("key2".to_string(), 454)));

        Ok(())
    }
}
