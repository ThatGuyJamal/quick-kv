#[cfg(feature = "full")]
use rayon::prelude::*;

use crate::types::BinaryKv;
use bincode::deserialize_from;
use serde::de::DeserializeOwned;
use serde::Serialize;
use std::collections::HashMap;
use std::fmt::Debug;
use std::fs::{File, OpenOptions};
use std::hash::Hash;
use std::io::{self, Seek, SeekFrom};
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

        // Seek to the beginning of the file
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
        todo!()
    }

    pub fn delete(&mut self, key: &str) -> std::io::Result<()> {
        todo!()
    }

    pub fn update(&mut self, key: &str, value: T) -> std::io::Result<()> {
        todo!()
    }

    pub fn clear(&mut self) -> std::io::Result<()> {
        todo!()
    }

    pub fn get_all(&mut self) -> std::io::Result<Vec<T>> {
        todo!()
    }

    pub fn get_many(&mut self, keys: Vec<String>) -> std::io::Result<Vec<T>> {
        todo!()
    }

    pub fn set_many(&mut self, values: Vec<BinaryKv<T>>) -> std::io::Result<()> {
        todo!()
    }

    pub fn delete_many(&mut self, keys: Vec<String>) -> std::io::Result<()> {
        todo!()
    }

    pub fn update_many(&mut self, values: Vec<BinaryKv<T>>) -> std::io::Result<()> {
        todo!()
    }
}
