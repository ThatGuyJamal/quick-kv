use std::collections::HashMap;
use std::fmt::Debug;
use std::fs::{File, OpenOptions};
use std::io::{self, Seek, SeekFrom, Write};
use std::sync::{Arc, Mutex};

use bincode::deserialize_from;
use serde::de::DeserializeOwned;
use serde::Serialize;

use crate::types::binarykv::{BinaryKv, BinaryKvCache};
use crate::utils::validate_database_file_path;

/// The Mini Client. Used for simple data storage and retrieval.
///
/// # Example
/// ```rust
/// use std::collections::HashMap;
///
/// use quick_kv::prelude::*;
///
/// let mut client = QuickClientMini::new(None).unwrap();
///
/// let mut map = HashMap::new();
///
/// for i in 0..9 {
///     map.insert(i.to_string(), i);
/// }
///
/// client
///     .set("test-hash", TypedValue::<i32>::Hash(map.clone()))
///     .unwrap();
///
/// let map_results = client
///     .get::<TypedValue<i32>>("test-hash")
///     .unwrap()
///     .unwrap()
///     .into_hash();
///
/// for (key, value) in map_results.iter() {
///     println!("{}: {}", key, value)
/// }
///
/// assert_eq!(map, map_results);
/// ```
#[derive(Debug)]
pub struct QuickClientMini
{
    pub file: Arc<Mutex<File>>,
    pub cache: Arc<Mutex<HashMap<String, BinaryKvCache>>>,
}

impl QuickClientMini
{
    /// Creates a new instance of the client.
    ///
    /// `path` to the database file. If `None` is provided, the database will be created in the current working directory
    /// and default to `db.qkv`.
    ///
    /// You can have as many client instances as you want, however, if you have multiple instances of the same client,
    /// you need to make sure they write to different databases or else there will be data corruption.
    pub fn new(path: Option<&str>) -> io::Result<Self>
    {
        let path = validate_database_file_path(path.unwrap_or("db.qkv"));

        let file = match OpenOptions::new().read(true).write(true).create(true).open(path) {
            Ok(file) => file,
            Err(e) => {
                return Err(io::Error::new(io::ErrorKind::Other, format!("Error opening file: {:?}", e)));
            }
        };

        Ok(Self {
            file: Arc::new(Mutex::new(file)),
            cache: Arc::new(Mutex::new(HashMap::new())),
        })
    }

    /// Get a value from the database.
    ///
    /// `key` to get the value for.
    ///
    /// Returns `Some(T)` if the key exists, `None` if the key does not exist.
    /// ```rust
    /// use quick_kv::prelude::*;
    ///
    /// let mut client = QuickClientMini::new(None).unwrap();
    ///
    /// let result = client.get::<i32>("doesnotexist").unwrap();
    ///
    /// assert_eq!(result, None);
    /// ```
    pub fn get<T>(&mut self, key: &str) -> io::Result<Option<T>>
    where
        T: Serialize + DeserializeOwned + Clone + Debug,
    {
        {
            let cache = match self.cache.lock() {
                Ok(cache) => cache,
                Err(e) => {
                    return Err(io::Error::new(io::ErrorKind::Other, format!("Error locking cache: {:?}", e)));
                }
            };

            if let Some(cache) = cache.get(key) {
                // We need to convert the cached binary data into the type we want. This is kinda unsafe and a hacky way to have caching but
                // If works for now. Will look for better solutions in the future.
                let deserialized_cache: T = match bincode::deserialize(&cache.value) {
                    Ok(data) => data,
                    Err(e) => {
                        return Err(io::Error::new(
                            io::ErrorKind::Other,
                            format!("Error deserializing data from cache: {:?}", e),
                        ));
                    }
                };
                return Ok(Some(deserialized_cache));
            }
        }

        let mut file = match self.file.lock() {
            Ok(file) => file,
            Err(e) => {
                return Err(io::Error::new(io::ErrorKind::Other, format!("Error locking file: {:?}", e)));
            }
        };

        let mut reader = io::BufReader::new(&mut *file);
        // Seek to the beginning of the file
        reader.seek(SeekFrom::Start(0))?;

        // Read and deserialize entries until the end of the file is reached
        loop {
            match deserialize_from::<_, BinaryKv<T>>(&mut reader) {
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

    /// Set a value in the database.
    ///
    /// `key` to set the value for.
    ///
    /// `value` to set for the key.
    ///
    /// ```rust
    /// use quick_kv::prelude::*;
    ///
    /// let mut client = QuickClientMini::new(None).unwrap();
    ///
    /// client.set("five", Value::I32(5).into_i32()).unwrap();
    ///
    /// let five = client.get::<i32>("five").unwrap().unwrap();
    ///
    /// assert_eq!(five, 5);
    /// ```
    pub fn set<T>(&mut self, key: &str, value: T) -> io::Result<()>
    where
        T: Serialize + DeserializeOwned + Clone + Debug,
    {
        {
            // If the key exists, update the value instead of adding a new key-value pair
            if self.cache.lock().unwrap().get(key).is_some() {
                return self.update(key, value);
            }
        }

        // Key doesn't exist, add a new key-value pair
        let mut file = match self.file.lock() {
            Ok(file) => file,
            Err(e) => {
                return Err(io::Error::new(io::ErrorKind::Other, format!("Error locking file: {:?}", e)));
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
        writer.get_ref().sync_all()?;

        let serialize_cache = bincode::serialize(&value).unwrap();

        self.cache.lock().unwrap().insert(
            key.to_string(),
            BinaryKvCache {
                key: key.to_string(),
                value: serialize_cache,
            },
        );

        Ok(())
    }

    /// Delete a value from the database.
    ///
    /// `key` to delete the value for.
    ///
    /// ```rust
    /// use quick_kv::prelude::*;
    ///
    /// let mut client = QuickClientMini::new(None).unwrap();
    ///
    /// client.set("five", Value::I32(5).into_i32()).unwrap();
    ///
    /// client.delete::<i32>("five").unwrap();
    ///
    /// let should_not_exist = client.get::<i32>("five").unwrap();
    ///
    /// assert_eq!(should_not_exist, None);
    /// ```
    pub fn delete<T>(&mut self, key: &str) -> io::Result<()>
    where
        T: Serialize + DeserializeOwned + Clone + Debug,
    {
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
        writer.get_mut().set_len(0)?;
        writer.seek(SeekFrom::Start(0))?;
        writer.flush()?;
        writer.get_ref().sync_all()?;

        self.cache.lock().unwrap().remove(key);

        Ok(())
    }

    /// Update a value in the database.
    ///
    /// `key` to update the value for.
    ///
    /// `value` to update for the key.
    ///
    /// ```rust
    /// use quick_kv::prelude::*;
    ///
    /// let mut client = QuickClientMini::new(None).unwrap();
    ///
    /// client.set("five", Value::I32(5).into_i32()).unwrap();
    /// let five = client.get::<i32>("five").unwrap().unwrap();
    /// assert_eq!(five, 5);
    ///
    /// client.update("five", 10).unwrap();
    /// let ten = client.get::<i32>("five").unwrap().unwrap();
    /// assert_eq!(ten, 10);
    /// ```
    pub fn update<T>(&mut self, key: &str, value: T) -> io::Result<()>
    where
        T: Serialize + DeserializeOwned + Clone + Debug,
    {
        {
            // If the value does not exist in cache, then we can set it and not update
            if self.cache.lock().unwrap().get(key).is_none() {
                return self.set(key, value);
            }
        }

        // Lock the file and use a buffered reader
        let mut file = match self.file.lock() {
            Ok(file) => file,
            Err(e) => {
                return Err(io::Error::new(io::ErrorKind::Other, format!("Error locking file: {:?}", e)));
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
            return Err(io::Error::new(io::ErrorKind::Other, format!("Key not found: {}", key)));
        }

        // Close the file and open it in write mode
        drop(reader); // Release the reader

        // Reopen the file in write mode for writing
        let mut writer = io::BufWriter::new(&mut *file);

        // Truncate the file and write the updated data back
        writer.get_mut().set_len(0)?;
        writer.seek(SeekFrom::Start(0))?;

        let mut serialized = Vec::new();

        for entry in updated_entries.iter() {
            match bincode::serialize(entry) {
                Ok(data) => {
                    serialized.extend_from_slice(&data);
                }
                Err(e) => panic!("Error serializing data: {:?}", e),
            };
        }

        writer.write_all(&serialized)?;
        writer.flush()?;
        writer.get_ref().sync_all()?;

        let serialize_cache = bincode::serialize(&value).unwrap();

        self.cache
            .lock()
            .unwrap()
            .insert(key.to_string(), BinaryKvCache::new(key.to_string(), serialize_cache));

        Ok(())
    }
}

#[cfg(test)]
mod tests
{
    use std::collections::HashMap;

    use tempfile::tempdir;

    use crate::prelude::*;

    #[test]
    fn test_set()
    {
        let tmp_dir = tempdir().expect("Failed to create tempdir");
        let tmp_file = tmp_dir.path().join("test.qkv");

        let mut client = QuickClientMini::new(Some(tmp_file.to_str().unwrap())).unwrap();

        let value = String::from("Hello World!");
        client.set("hello", value).unwrap();
    }

    #[test]
    fn test_set_multiple_keys_with_same_name()
    {
        let tmp_dir = tempdir().expect("Failed to create tempdir");
        let tmp_file = tmp_dir.path().join("test.qkv");

        let mut client = QuickClientMini::new(Some(tmp_file.to_str().unwrap())).unwrap();

        // Set the initial value for the key
        client.set("hello9", String::from("Hello World!")).unwrap();

        // Verify that the initial value is correct
        let result = client.get::<String>("hello9").unwrap();
        assert_eq!(result, Some(String::from("Hello World!")));

        // Set a new value for the same key
        client.set("hello9", String::from("Updated Value")).unwrap();

        // Verify that the value has been updated
        let result2 = client.get::<String>("hello9").unwrap();
        assert_eq!(result2, Some(String::from("Updated Value")));
    }

    #[test]
    fn test_get()
    {
        let tmp_dir = tempdir().expect("Failed to create tempdir");
        let tmp_file = tmp_dir.path().join("test.qkv");

        let mut client = QuickClientMini::new(Some(tmp_file.to_str().unwrap())).unwrap();

        let value = String::from("Hello World!");
        client.set("hello2", value.clone()).unwrap();

        let result = client.get::<String>("hello2").unwrap();
        assert_eq!(result, Some(value));
    }

    #[test]
    fn test_get_not_found()
    {
        let tmp_dir = tempdir().expect("Failed to create tempdir");
        let tmp_file = tmp_dir.path().join("test.qkv");

        let mut client = QuickClientMini::new(Some(tmp_file.to_str().unwrap())).unwrap();

        let value = String::from("Hello World!");
        client.set("hello3", value).unwrap();

        let result = client.get::<String>("doesnotexist-124319284791827948179").unwrap();
        assert_eq!(result, None);
    }

    #[test]
    fn test_get_multiple()
    {
        let tmp_dir = tempdir().expect("Failed to create tempdir");
        let tmp_file = tmp_dir.path().join("test.qkv");

        let mut client = QuickClientMini::new(Some(tmp_file.to_str().unwrap())).unwrap();
        let value = String::from("Hello World!");

        client.set("hello5", value.clone()).unwrap();
        client.set("hello6", value.clone()).unwrap();

        let result = client.get::<String>("hello5").unwrap();
        assert_eq!(result, Some(value)); // Clone the value to compare it
    }

    #[test]
    fn test_delete()
    {
        let tmp_dir = tempdir().expect("Failed to create tempdir");
        let tmp_file = tmp_dir.path().join("test.qkv");

        let mut client = QuickClientMini::new(Some(tmp_file.to_str().unwrap())).unwrap();
        let value = String::from("Hello World!");

        client.set("hello7", value.clone()).unwrap();
        let result = client.get::<String>("hello7").unwrap();
        assert_eq!(result, Some(value.clone()));

        client.delete::<String>("hello7").unwrap();

        let result = client.get::<String>("hello7").unwrap();
        assert_eq!(result, None);
    }

    #[test]
    fn test_update()
    {
        let tmp_dir = tempdir().expect("Failed to create tempdir");
        let tmp_file = tmp_dir.path().join("test.qkv");

        let mut client = QuickClientMini::new(Some(tmp_file.to_str().unwrap())).unwrap();

        client.set::<String>("hello8", String::from("Hello World!")).unwrap();

        let result = client.get::<String>("hello8").unwrap();
        assert_eq!(result, Some(String::from("Hello World!")));

        client.update::<String>("hello8", String::from("Hello World! 2")).unwrap();

        let result2 = client.get::<String>("hello8").unwrap();
        assert_eq!(result2, Some(String::from("Hello World! 2")));
    }

    #[test]
    fn test_vector_injection()
    {
        let tmp_dir = tempdir().expect("Failed to create tempdir");
        let tmp_file = tmp_dir.path().join("test.qkv");

        let mut client = QuickClientMini::new(Some(tmp_file.to_str().unwrap())).unwrap();

        let mut v = Vec::new();

        for i in 0..9 {
            v.push(i);
        }

        client.set("vec", TypedValue::<i32>::Vec(v.clone())).unwrap();

        let result = client.get::<TypedValue<i32>>("vec").unwrap().unwrap().into_vec();

        for i in 0..9 {
            assert_eq!(result[i], v[i]);
        }

        assert_eq!(result.len(), v.len());
    }

    #[test]
    fn test_hashmap_injection()
    {
        let tmp_dir = tempdir().expect("Failed to create tempdir");
        let tmp_file = tmp_dir.path().join("test.qkv");

        let mut client = QuickClientMini::new(Some(tmp_file.to_str().unwrap())).unwrap();

        let mut map = HashMap::new();

        for i in 0..4 {
            map.insert(i.to_string(), i);
        }

        client.set("map", TypedValue::<i32>::Hash(map.clone())).unwrap();

        let result = client.get::<TypedValue<i32>>("map").unwrap().unwrap().into_hash();

        assert_eq!(result.len(), map.len());
    }
}
