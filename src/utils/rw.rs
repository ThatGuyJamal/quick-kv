use crate::utils::types::{QuickKVConfig, Value};
use std::fs::File;
use std::io::{self, Write, Read, Seek};

#[derive(Debug)]
pub struct ReaderWriter {
    pub file: File,
}

impl ReaderWriter {
    pub fn new(config: QuickKVConfig) -> Self {
        let file_path = match config.db_file {
            Some(file_path) => file_path,
            None => panic!("No db file specified in config"),
        };

        let file = match File::open(&file_path) {
            Ok(file) => file,
            Err(_) => {
                match File::create(&file_path) {
                    Ok(file) => file,
                    Err(e) => panic!("Error creating db file: {}", e),
                }
            }
        };

        Self { file }
    }

    pub fn write(&mut self, key: &str, value: &Value) -> io::Result<()> {
        let key_len = key.len() as u32;
        let value_bytes = bincode::serialize(value)
            .map_err(|e| io::Error::new(io::ErrorKind::InvalidInput, e))?;
        let value_len = value_bytes.len() as u32;

        // Write the key length and value length as 4-byte integers
        self.file.write_all(&key_len.to_be_bytes())?;
        self.file.write_all(&value_len.to_be_bytes())?;

        // Write the key and value
        self.file.write_all(key.as_bytes())?;
        self.file.write_all(&value_bytes)?;

        Ok(())
    }


    pub fn read(&mut self, key: &str) -> io::Result<Option<Value>> {
        loop {
            // Read key length and value length
            let mut key_len_bytes = [0u8; 4];
            let mut value_len_bytes = [0u8; 4];

            if self.file.read_exact(&mut key_len_bytes).is_err() {
                break; // Exit loop when there's no more data to read
            }

            self.file.read_exact(&mut value_len_bytes)?;

            let key_len = u32::from_be_bytes(key_len_bytes);
            let value_len = u32::from_be_bytes(value_len_bytes);

            // Read the key and value
            let mut key_bytes = vec![0u8; key_len as usize];
            self.file.read_exact(&mut key_bytes)?;

            println!("key_bytes: {:?}", key_bytes);
            println!("key: {:?}", key.as_bytes());

            if key_bytes == key.as_bytes() {
                let mut value_bytes = vec![0u8; value_len as usize];
                self.file.read_exact(&mut value_bytes)?;

                println!("value_bytes: {:?}", value_bytes);

                // Deserialize the value using bincode
                return Ok(Some(bincode::deserialize(&value_bytes)
                    .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?));
            } else {
                // Skip the value since it doesn't match the requested key
                self.file.seek(io::SeekFrom::Current(value_len as i64))?;
            }
        }

        Ok(None)
    }

}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_write_and_read_value() {
        // Create a QuickKVConfig with the temporary file path
        let config = QuickKVConfig {
            db_file: "db.qkv".to_string().into(),
            max_db_size: None,
        };

        // Create a ReaderWriter instance
        let mut reader_writer = ReaderWriter::new(config);

        // Define a key and value to write
        let key = "test_key";
        let value = Value::String("test_value".to_string());

        // Write the key and value
        let write_result = reader_writer.write(key, &value);
        assert!(write_result.is_ok());

        // Read the value using the same key
        let read_result = reader_writer.read(key);
        assert!(read_result.is_ok());

        println!("{:?}", read_result);

        // Ensure the read result matches the expected value
        // todo - fix error with read function returning None
        let read_value = read_result.unwrap().unwrap();
        assert_eq!(read_value, value);

        std::fs::remove_file("db.qkv").unwrap();
    }

    #[test]
    fn test_read_nonexistent_key() {
        // Create a QuickKVConfig with the temporary file path
        let config = QuickKVConfig {
            db_file: "db.qkv".to_string().into(),
            max_db_size: None,
        };

        // Create a ReaderWriter instance
        let mut reader_writer = ReaderWriter::new(config);

        // Attempt to read a key that doesn't exist
        let read_result = reader_writer.read("nonexistent_key");
        assert!(read_result.is_ok());

        // Ensure the read result is None
        let read_option = read_result.unwrap();
        assert_eq!(read_option, None);

        std::fs::remove_file("db.qkv").unwrap();
    }
}
