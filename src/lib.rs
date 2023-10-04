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

use std::fmt::Debug;
use std::fs::{File, OpenOptions};
use std::io::{self, Read, Seek, Write};
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use serde::{Deserialize, Deserializer, Serialize};
use bincode::deserialize_from;
use serde::de::DeserializeOwned;

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
        T: Serialize + Clone  + Debug,
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
            None => {
                PathBuf::from("quick.db")
            }
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
        let mut file = match self.file.lock() {
            Ok(file) => file,
            Err(e) => {
                return Err(io::Error::new(
                    io::ErrorKind::Other,
                    format!("Error locking file: {:?}", e),
                ));
            }
        };

        let data = BinaryKv::new(key.to_string(), value.clone());

        let serialized = match bincode::serialize(&data) {
            Ok(data) => data,
            Err(e) => panic!("Error serializing data: {:?}", e),
        };

        file.write_all(&serialized)?;

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

        let mut buffer: Vec<u8> = Vec::new();

        // Read the entire contents of the file into the buffer
        file.seek(io::SeekFrom::Start(0))?; // Move to the beginning of the file
        file.read_to_end(&mut buffer)?;

        // Deserialize each entry in the buffer and find the matching key
        let mut cursor = io::Cursor::new(&buffer);
        while cursor.position() < cursor.get_ref().len() as u64 {
            match deserialize_from::<_, BinaryKv<T>>(&mut cursor) {
                Ok(BinaryKv { key: entry_key, value }) if key == entry_key => {
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

        let mut buffer: Vec<u8> = Vec::new();
        let mut updated_buffer: Vec<u8> = Vec::new();

        // Read the entire contents of the file into the buffer
        file.seek(io::SeekFrom::Start(0))?; // Move to the beginning of the file
        file.read_to_end(&mut buffer)?;

        // Deserialize each entry in the buffer and check for the matching key
        let mut cursor = io::Cursor::new(&buffer);
        while cursor.position() < cursor.get_ref().len() as u64 {
            match deserialize_from::<_, BinaryKv<T>>(&mut cursor) {
                Ok(BinaryKv { key: entry_key, .. }) if key != entry_key => {
                    // Keep entries that don't match the key
                    updated_buffer.extend_from_slice(cursor.get_ref());
                    break;
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

        // Truncate the file and write the updated data back
        file.set_len(0)?;
        file.seek(io::SeekFrom::Start(0))?;
        file.write_all(&updated_buffer)?;

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
        let mut file = match self.file.lock() {
            Ok(file) => file,
            Err(e) => {
                return Err(io::Error::new(
                    io::ErrorKind::Other,
                    format!("Error locking file: {:?}", e),
                ));
            }
        };

        let mut buffer: Vec<u8> = Vec::new();

        // Read the entire contents of the file into the buffer
        file.seek(io::SeekFrom::Start(0))?; // Move to the beginning of the file
        file.read_to_end(&mut buffer)?;

        let mut cursor = io::Cursor::new(&mut buffer);
        let mut updated_entries: Vec<BinaryKv<T>> = Vec::new();
        let mut updated = false;

        while cursor.position() < cursor.get_ref().len() as u64 {
            match deserialize_from::<_, BinaryKv<T>>(&mut cursor) {
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

        // If no updates were made, return an error
        if !updated {
            return Err(io::Error::new(
                io::ErrorKind::NotFound,
                format!("Key '{}' not found for update", key),
            ));
        }

        // Clear the file contents and write the updated entries back to the file
        file.seek(io::SeekFrom::Start(0))?; // Move to the beginning of the file
        file.set_len(0)?; // Clear the file contents
        for entry in updated_entries.iter() {
            let serialized = match bincode::serialize(entry) {
                Ok(data) => data,
                Err(e) => panic!("Error serializing data: {:?}", e),
            };
            file.write_all(&serialized)?;
        }

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

        let result = client.get::<String>("doesnotexist-124319284791827948179").unwrap();
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

        client.set::<String>("hello8", String::from("Hello World!")).unwrap();
        let result = client.get::<String>("hello8").unwrap();
        assert_eq!(result, Some(String::from("Hello World!")));

        client.update::<String>("hello8", String::from("Hello World! 2")).unwrap();
        let result2 = client.get::<String>("hello8").unwrap();
        assert_eq!(result2, Some(String::from("Hello World! 2")));
    }
}