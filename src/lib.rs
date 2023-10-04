use std::fs::{File, OpenOptions};
use std::io::{self, Read, Seek, Write};
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use serde::{Deserialize, Deserializer, Serialize};
use bincode::deserialize_from;
use serde::de::DeserializeOwned;

#[derive(Debug)]
pub struct QuickClient {
    file: Arc<Mutex<File>>,
}

#[derive(Serialize, PartialEq, Debug, Clone)]
pub struct BinaryKv<T>
    where
        T: Serialize + Clone,
{
    pub key: String,
    pub value: T,
}

impl<T> BinaryKv<T>
    where
        T: Serialize + Clone,
{
    fn new(key: String, value: T) -> Self {
        BinaryKv { key, value }
    }
}

impl<'de, T> Deserialize<'de> for BinaryKv<T>
    where
        T: Deserialize<'de> + Serialize + Clone,
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
    pub fn new(path: PathBuf) -> io::Result<Self> {
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
        })
    }

    pub fn set<T>(&mut self, key: &str, value: T) -> io::Result<()>
        where
            T: Serialize + DeserializeOwned + Clone,
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

    pub fn get<T>(&mut self, key: &str) -> io::Result<Option<T>>
        where
            T: Serialize + DeserializeOwned + Clone,
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

}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_set() {
        let tmp_dir = tempdir().expect("Failed to create tempdir");
        let tmp_file = tmp_dir.path().join("test.qkv");
        let mut client = QuickClient::new(tmp_file).unwrap();
        let value = String::from("Hello World!");
        client.set::<String>("hello", value).unwrap();
    }

    #[test]
    fn test_get() {
        let tmp_dir = tempdir().expect("Failed to create tempdir");
        let tmp_file = tmp_dir.path().join("test.qkv");
        let mut client = QuickClient::new(tmp_file).unwrap();
        let value = String::from("Hello World!");
        client.set::<String>("hello2", value.clone()).unwrap();

        let result = client.get("hello2").unwrap();
        assert_eq!(result, Some(value));
    }

    #[test]
    fn test_get_not_found() {
        let tmp_dir = tempdir().expect("Failed to create tempdir");
        let tmp_file = tmp_dir.path().join("test.qkv");
        let mut client = QuickClient::new(tmp_file).unwrap();
        let value = String::from("Hello World!");
        client.set::<String>("hello3", value).unwrap();

        let result = client.get::<String>("doesnotexist-124319284791827948179").unwrap();
        assert_eq!(result, None);
    }

    #[test]
    fn test_get_multiple() {
        let tmp_dir = tempdir().expect("Failed to create tempdir");
        let tmp_file = tmp_dir.path().join("test.qkv");
        let mut client = QuickClient::new(tmp_file).unwrap();
        let value = String::from("Hello World!");
        client.set::<String>("hello5", value.clone()).unwrap();
        client.set::<String>("hello6", value.clone()).unwrap();

        let result = client.get::<String>("hello5").unwrap();
        assert_eq!(result, Some(value)); // Clone the value to compare it
    }

}