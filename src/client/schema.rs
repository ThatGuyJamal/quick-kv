use rayon::prelude::*;

use crate::types::BinaryKv;
use bincode::deserialize_from;
use serde::de::DeserializeOwned;
use serde::Serialize;
use std::collections::HashMap;
use std::fmt::Debug;
use std::fs::{File, OpenOptions};
use std::hash::Hash;
use std::io::{self, Seek, SeekFrom, Write};
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

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
}

impl<T> QuickSchemaClient<T>
where
    T: Serialize + DeserializeOwned + Clone + Debug + Eq + PartialEq + Hash,
{
    pub fn new(path: Option<PathBuf>) -> std::io::Result<Self> {
        let path = match path {
            Some(path) => path,
            None => PathBuf::from("db.qkv"),
        };

        let file = match OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .open(&path)
        {
            Ok(file) => file,
            Err(e) => {
                return Err(io::Error::new(
                    io::ErrorKind::Other,
                    format!("Error opening file: {:?}", e),
                ));
            }
        };

        Ok(Self {
            file: Arc::new(Mutex::new(file)),
            cache: Mutex::new(HashMap::new()),
            position: 0,
        })
    }

    pub fn get(&mut self, key: &str) -> std::io::Result<Option<T>> {
        // Check if the key is in the cache first
        let cache = self.cache.lock().unwrap();
        if let Some(entry) = cache.get(key) {
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

                    // Update the current position
                    self.position = reader.seek(SeekFrom::Current(0))?;

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

        // Key not found
        Ok(None)
    }

    pub fn set(&mut self, key: &str, value: T) -> std::io::Result<()> {
        // First check if the data already exist, if so, update it not set it again.
        // This will stop memory alloc errors.
        if self.cache.lock().unwrap().get(key).is_some() {
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

        Ok(())
    }

    pub fn delete(&mut self, key: &str) -> std::io::Result<()> {
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

        Ok(())
    }

    pub fn update(&mut self, key: &str, value: T) -> std::io::Result<()> {
        if self.cache.lock().unwrap().get(key).is_none() {
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

        Ok(())
    }

    pub fn clear(&mut self) -> std::io::Result<()> {
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

        Ok(())
    }

    pub fn get_all(&mut self) -> std::io::Result<Vec<BinaryKv<T>>> {
        let all_results = self
            .cache
            .lock()
            .unwrap()
            .iter()
            .map(|(_, entry)| entry.clone())
            .collect();

        Ok(all_results)
    }

    pub fn get_many(&mut self, keys: Vec<String>) -> std::io::Result<Vec<T>> {
        let mut results = Vec::new();

        for key in keys {
            if let Some(entry) = self.cache.lock().unwrap().get(&key) {
                results.push(entry.value.clone());
            }
        }

        Ok(results)
    }

    pub fn set_many(&mut self, values: Vec<BinaryKv<T>>) -> std::io::Result<()> {
        // First check if the data already exist, if so, update it not set it again.
        // This will stop memory alloc errors.
        let mut to_update = Vec::new();

        for entry in values.iter() {
            if self.cache.lock().unwrap().get(&entry.key).is_some() {
                to_update.push(entry.clone());
            }
        }

        if !to_update.is_empty() {
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
        writer.get_ref().sync_all()?;

        for entry in values.iter() {
            self.cache.lock().unwrap().insert(
                entry.key.clone(),
                BinaryKv::new(entry.key.clone(), entry.value.clone()),
            );
        }

        Ok(())
    }

    pub fn delete_many(&mut self, keys: Vec<String>) -> std::io::Result<()> {

        if self.cache.lock().unwrap().is_empty() {
            return Ok(());
        }

        // First we check if any of the keys passed exist, before we search the file for them.
        let mut valid_keys = Vec::new();
        for key in keys {
            if self.cache.lock().unwrap().get(&key).is_some() {
                valid_keys.push(key);
            }
        }

        if valid_keys.is_empty() {
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

        Ok(())
    }

    pub fn update_many(&mut self, values: Vec<BinaryKv<T>>) -> std::io::Result<()> {
        let mut to_set = Vec::new();

        for entry in values.iter() {
            if self.cache.lock().unwrap().get(&entry.key).is_none() {
                to_set.push(entry.clone());
            }
        };

        if !to_set.is_empty() {
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

        // Truncate the file and write the updated data back
        writer.get_mut().set_len(0)?;
        writer.seek(SeekFrom::Start(0))?;
        writer.write_all(&serialized)?;
        writer.get_ref().sync_all()?;

        for entry in updated_entries.iter() {
            self.cache.lock().unwrap().insert(
                entry.key.clone(),
                BinaryKv::new(entry.key.clone(), entry.value.clone()),
            );
        }

        Ok(())
    }
}
