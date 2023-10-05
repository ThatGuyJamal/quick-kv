use bincode::deserialize_from;
use serde::de::DeserializeOwned;
use serde::{Serialize};
use std::fmt::Debug;
use std::fs::{File, OpenOptions};
use std::io::{self, Seek, SeekFrom, Write};
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use crate::BinaryKv;

/// The client for the QuickKV database
#[derive(Debug)]
pub struct QuickClient {
    file: Arc<Mutex<File>>,
}

impl QuickClient {
    pub fn new(path: Option<PathBuf>) -> io::Result<Self> {
        let path = match path {
            Some(path) => path,
            None => PathBuf::from("db.qkv"),
        };

        let file = match OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .open(path)
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
        })
    }

    pub fn get<T>(&mut self, key: &str) -> io::Result<Option<T>>
        where
            T: Serialize + DeserializeOwned + Clone + Debug,
    {
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
        reader.seek(SeekFrom::Start(0))?;

        // Read and deserialize entries until the end of the file is reached
        loop {
            match deserialize_from::<_, BinaryKv<T>>(&mut reader) {
                Ok(BinaryKv {
                       key: entry_key,
                       value,
                   }) if key == entry_key => {
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

    pub fn set<T>(&mut self, key: &str, value: T) -> io::Result<()>
        where
            T: Serialize + DeserializeOwned + Clone + Debug,
    {
        if self.get::<T>(key)?.is_none() {
            // Key doesn't exist, add a new key-value pair
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
                Err(e) => panic!("Error serializing data: {:?}", e),
            };

            // Write the serialized data to the file
            writer.write_all(&serialized)?;

            // Flush the writer to ensure data is written to the file
            writer.flush()?;
        } else {
            // Key already exists, update the value
            self.update(key, value)?;
        }

        Ok(())
    }

    /// Deletes a key-value pair from the database
    ///
    /// `key` The key of the key-value pair
    ///
    /// # Examples
    /// ```rust
    ///  use quick_kv::{QuickClient, Value};
    ///
    ///  let mut client = QuickClient::new(None).unwrap();
    ///
    ///  client.set("hello", Value::String("hello world!".to_string())).unwrap();
    ///
    ///  client.delete::<Value>("hello").unwrap();
    ///
    ///  let result = client.get::<Value>("hello").unwrap();
    ///
    ///  assert_eq!(result, None);
    /// ```
    pub fn delete<T>(&mut self, key: &str) -> io::Result<()>
        where
            T: Serialize + DeserializeOwned + Clone + Debug,
    {
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
        writer.get_mut().set_len(0)?;
        writer.seek(SeekFrom::Start(0))?;
        writer.write_all(&updated_buffer)?;

        // Flush the writer to ensure data is written to the file
        writer.flush()?;

        Ok(())
    }

    /// Updates a key-value pair in the database
    ///
    /// `key` The key of the key-value pair
    ///
    /// `value` The new value of the key-value pair
    ///
    /// If no key is found, an error is returned and the database is not updated.
    ///
    /// # Examples
    /// ```rust
    ///  use quick_kv::{QuickClient, Value};
    ///
    ///  let mut client = QuickClient::new(None).unwrap();
    ///
    ///  client.set("hello", Value::String("hello world!".to_string())).unwrap();
    ///
    ///  client.update::<Value>("hello", Value::String("hello world! 2".to_string())).unwrap();
    ///
    ///  let result = client.get::<Value>("hello").unwrap();
    ///
    ///  assert_eq!(result, Some(Value::String("hello world! 2".to_string())));
    /// ```
    pub fn update<T>(&mut self, key: &str, value: T) -> io::Result<()>
        where
            T: Serialize + DeserializeOwned + Clone + Debug,
    {
        // Lock the file and use a buffered reader
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
                Err(e) => panic!("Error serializing data: {:?}", e),
            };
            writer.write_all(&serialized)?;
        }

        // Flush the writer to ensure data is written to the file
        writer.flush()?;

        Ok(())
    }
}