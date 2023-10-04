//! QuickKV is a simple key-value database written in Rust
//! Its goal is to allow thread safe access to a database file with minimal overhead
//! It is not meant to be a replacement for a full fledged database, but rather a simple way to store data.
//!
//! # Features
//!
//! - Thread safe
//! - Simple API
//! - Minimal overhead
//! - Supports any type that implements `Serialize` and `Deserialize` from the `serde` crate
//!
//! ## Supported Crud Operations
//!
//! - Set
//! - Get
//! - Update
//! - Delete
//!
//! # Installation
//!
//! ```bash
//! cargo add quick-kv
//! ```
//!
//! # Why use QuickKV?
//!
//! QuickKV is meant to be used in situations where you need to store data, but don't want to deal with the overhead of a full fledged database.
//! It is also useful for storing data that doesn't need to be accessed very often, but still needs to be stored. QuickKV also has the benefit of
//! storing data in a file as binary making it faster than other file formats for data storage.
//!
//! # Examples
//! ```rust
//! use quick_kv::QuickClient;
//!
//! fn main() {
//!    let mut client = QuickClient::new(None).unwrap();
//!
//!    client.set::<String>("hello", String::from("Hello World!")).unwrap();
//!    let result = client.get::<String>("hello").unwrap();
//!
//!    assert_eq!(result, Some(String::from("Hello World!")));
//! }
//! ```

use bincode::deserialize_from;
use serde::de::DeserializeOwned;
use serde::{Deserialize, Deserializer, Serialize};
use std::fmt::Debug;
use std::fs::{File, OpenOptions};
use std::io::{self, Seek, SeekFrom, Write};
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

/// The client for the QuickKV database
///
/// # Examples
/// ```rust
/// use quick_kv::QuickClient;
///
/// fn main() {
///     let mut client = QuickClient::new(None).unwrap();
///
///     client.set::<String>("hello", String::from("Hello World!")).unwrap();
///
///     let result = client.get::<String>("hello").unwrap();
///
///     assert_eq!(result, Some(String::from("Hello World!")));
/// }
#[derive(Debug)]
pub struct QuickClient {
    file: Arc<Mutex<File>>,
}

/// The data structure used to store key-value pairs in the database
#[derive(Serialize, PartialEq, Debug, Clone)]
pub struct BinaryKv<T>
where
    T: Serialize + Clone + Debug,
{
    /// The key of the key-value pair
    pub key: String,
    /// The value of the key-value pair
    ///
    /// This is stored as a generic type so that any type can be stored
    ///
    /// # Examples
    /// ```rust
    /// use quick_kv::BinaryKv;
    ///
    /// BinaryKv::<String> {
    ///    key: String::from("hello"),
    ///   value: String::from("Hello World!"),
    /// };
    /// ```
    pub value: T,
}

impl<T> BinaryKv<T>
where
    T: Serialize + Clone + Debug,
{
    fn new(key: String, value: T) -> Self {
        BinaryKv { key, value }
    }
}

impl<'de, T> Deserialize<'de> for BinaryKv<T>
where
    T: Deserialize<'de> + Serialize + Clone + Debug,
{
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        #[derive(Deserialize)]
        struct ValueHelper<T> {
            key: String,
            value: T,
        }

        let helper = ValueHelper::<T>::deserialize(deserializer)?;

        Ok(BinaryKv {
            key: helper.key,
            value: helper.value,
        })
    }
}

impl QuickClient {
    /// Creates a new QuickClient instance
    ///
    /// `path` The path to the database file
    ///
    /// # Examples
    /// ```rust
    /// use quick_kv::QuickClient;
    ///
    /// fn main() {
    ///     let mut client = QuickClient::new(None).unwrap();
    /// }
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

    /// Sets a key-value pair in the database
    ///
    /// `key` The key of the key-value pair
    ///
    /// `value` The value of the key-value pair
    ///
    /// # Examples
    /// ```rust
    /// use quick_kv::QuickClient;
    ///
    /// fn main() {
    ///    let mut client = QuickClient::new(None).unwrap();
    ///
    ///   client.set::<String>("hello", String::from("Hello World!")).unwrap();
    /// }
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

    /// Gets a value from the database
    ///
    /// `key` The key of the key-value pair
    ///
    /// # Examples
    /// ```rust
    /// use quick_kv::QuickClient;
    ///
    /// fn main() {
    ///   let mut client = QuickClient::new(None).unwrap();
    ///
    ///   client.set::<String>("hello", String::from("Hello World!")).unwrap();
    ///   let result = client.get::<String>("hello").unwrap();
    ///
    ///   assert_eq!(result, Some(String::from("Hello World!")));
    /// }
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

    /// Deletes a key-value pair from the database
    ///
    /// `key` The key of the key-value pair
    ///
    /// # Examples
    /// ```rust
    /// use quick_kv::QuickClient;
    ///
    /// fn main() {
    ///    let mut client = QuickClient::new(None).unwrap();
    ///
    ///    client.set::<String>("hello", String::from("Hello World!")).unwrap();
    ///
    ///    client.delete::<String>("hello").unwrap();
    ///
    ///    let result = client.get::<String>("hello").unwrap();
    ///
    ///    assert_eq!(result, None);
    /// }
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
    /// use quick_kv::QuickClient;
    ///
    /// fn main() {
    ///    let mut client = QuickClient::new(None).unwrap();
    ///
    ///   client.set::<String>("hello", String::from("Hello")).unwrap();
    ///
    ///   client.update::<String>("hello", String::from("Hello World!")).unwrap();    ///
    ///
    ///   let result = client.get::<String>("hello").unwrap();
    ///
    ///   assert_eq!(result, Some(String::from("Hello World!")));
    /// }
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
                    if key == &entry.key {
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

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_set() {
        let tmp_dir = tempdir().expect("Failed to create tempdir");
        let tmp_file = tmp_dir.path().join("test.qkv");

        let mut client = QuickClient::new(Some(tmp_file)).unwrap();

        let value = String::from("Hello World!");
        client.set::<String>("hello", value).unwrap();
    }

    #[test]
    fn test_set_multiple_keys_with_same_name() {
        let tmp_dir = tempdir().expect("Failed to create tempdir");
        let tmp_file = tmp_dir.path().join("test.qkv");

        let mut client = QuickClient::new(Some(tmp_file.clone())).unwrap();

        // Set the initial value for the key
        client
            .set::<String>("hello9", String::from("Hello World!"))
            .unwrap();

        // Verify that the initial value is correct
        let result = client.get::<String>("hello9").unwrap();
        assert_eq!(result, Some(String::from("Hello World!")));

        // Set a new value for the same key
        client
            .set::<String>("hello9", String::from("Updated Value"))
            .unwrap();

        // Verify that the value has been updated
        let result2 = client.get::<String>("hello9").unwrap();
        assert_eq!(result2, Some(String::from("Updated Value")));
    }

    #[test]
    fn test_get() {
        let tmp_dir = tempdir().expect("Failed to create tempdir");
        let tmp_file = tmp_dir.path().join("test.qkv");

        let mut client = QuickClient::new(Some(tmp_file)).unwrap();

        let value = String::from("Hello World!");
        client.set::<String>("hello2", value.clone()).unwrap();

        let result = client.get::<String>("hello2").unwrap();
        assert_eq!(result, Some(value));
    }

    #[test]
    fn test_get_not_found() {
        let tmp_dir = tempdir().expect("Failed to create tempdir");
        let tmp_file = tmp_dir.path().join("test.qkv");

        let mut client = QuickClient::new(Some(tmp_file)).unwrap();

        let value = String::from("Hello World!");
        client.set::<String>("hello3", value).unwrap();

        let result = client
            .get::<String>("doesnotexist-124319284791827948179")
            .unwrap();
        assert_eq!(result, None);
    }

    #[test]
    fn test_get_multiple() {
        let tmp_dir = tempdir().expect("Failed to create tempdir");
        let tmp_file = tmp_dir.path().join("test.qkv");

        let mut client = QuickClient::new(Some(tmp_file)).unwrap();
        let value = String::from("Hello World!");

        client.set::<String>("hello5", value.clone()).unwrap();
        client.set::<String>("hello6", value.clone()).unwrap();

        let result = client.get::<String>("hello5").unwrap();
        assert_eq!(result, Some(value)); // Clone the value to compare it
    }

    #[test]
    fn test_delete() {
        let tmp_dir = tempdir().expect("Failed to create tempdir");
        let tmp_file = tmp_dir.path().join("test.qkv");

        let mut client = QuickClient::new(Some(tmp_file)).unwrap();
        let value = String::from("Hello World!");

        client.set::<String>("hello7", value.clone()).unwrap();
        let result = client.get::<String>("hello7").unwrap();
        assert_eq!(result, Some(value.clone()));

        client.delete::<String>("hello7").unwrap();

        let result = client.get::<String>("hello7").unwrap();
        assert_eq!(result, None);
    }

    #[test]
    fn test_update() {
        let tmp_dir = tempdir().expect("Failed to create tempdir");
        let tmp_file = tmp_dir.path().join("test.qkv");

        let mut client = QuickClient::new(Some(tmp_file.clone())).unwrap();

        client
            .set::<String>("hello8", String::from("Hello World!"))
            .unwrap();

        let result = client.get::<String>("hello8").unwrap();
        assert_eq!(result, Some(String::from("Hello World!")));

        client
            .update::<String>("hello8", String::from("Hello World! 2"))
            .unwrap();

        let result2 = client.get::<String>("hello8").unwrap();
        assert_eq!(result2, Some(String::from("Hello World! 2")));
    }

    #[test]
    fn test_vector_injection() {
        let tmp_dir = tempdir().expect("Failed to create tempdir");
        let tmp_file = tmp_dir.path().join("test.qkv");

        let mut client = QuickClient::new(Some(tmp_file.clone())).unwrap();

        let mut v = Vec::new();

        for i in 0..100 {
            v.push(i);
        }

        client.set::<Vec<i32>>("vec", v.clone()).unwrap();

        let result = client.get::<Vec<i32>>("vec").unwrap().unwrap();

        for i in 0..100 {
            assert_eq!(result[i], v[i]);
        }

        assert_eq!(result.len(), v.len());
    }
}
