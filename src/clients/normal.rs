use std::fmt::Debug;
use std::hash::Hash;
use std::time::Instant;

use serde::de::DeserializeOwned;
use serde::Serialize;

use crate::clients::{BaseClient, ClientConfig};
use crate::db::config::DatabaseConfiguration;
use crate::db::runtime::{RunTime, RuntTimeType};
use crate::db::Database;

#[derive(Debug, Clone)]
pub struct QuickClient<T>
where
    T: Serialize + DeserializeOwned + Debug + Eq + PartialEq + Hash + Send + Sync + Clone + 'static,
{
    db: Database<T>,
}

impl<T> BaseClient<T> for QuickClient<T>
where
    T: Serialize + DeserializeOwned + Debug + Eq + PartialEq + Hash + Send + Sync + Clone + 'static,
{
    fn new(config: ClientConfig) -> Self
    {
        let _config = DatabaseConfiguration::new(
            config.path,
            Some(RunTime::new(RuntTimeType::Disk)),
            config.log,
            config.log_level,
            config.default_ttl,
        )
        .unwrap();

        let db = Database::new(_config).unwrap();

        Self { db }
    }

    fn get(&mut self, key: &str) -> anyhow::Result<Option<T>>
    {
        match self.db.get(key.to_string()) {
            Ok(value) => Ok(value),
            Err(e) => Err(e),
        }
    }

    fn set(&mut self, key: &str, value: T) -> anyhow::Result<()>
    {
        match self.db.set(key, value, None) {
            Ok(_) => Ok(()),
            Err(e) => Err(e),
        }
    }

    fn update(&mut self, key: &str, value: T, upsert: Option<bool>) -> anyhow::Result<()>
    {
        match self.db.update(key, value, None, upsert) {
            Ok(_) => Ok(()),
            Err(e) => Err(e),
        }
    }

    fn delete(&mut self, key: &str) -> anyhow::Result<()>
    {
        match self.db.delete(key) {
            Ok(_) => Ok(()),
            Err(e) => Err(e),
        }
    }

    fn exists(&mut self, key: &str) -> anyhow::Result<bool>
    {
        match self.db.state.lock().unwrap().entries.contains_key(key) {
            true => Ok(true),
            false => Ok(false),
        }
    }

    fn keys(&mut self) -> anyhow::Result<Option<Vec<String>>>
    {
        let keys = self.db.state.lock().unwrap().entries.keys().cloned().collect::<Vec<String>>();
        if !keys.is_empty() {
            Ok(Some(keys))
        } else {
            Ok(None)
        }
    }

    fn values(&mut self) -> anyhow::Result<Option<Vec<T>>>
    {
        let values = self.db.state.lock().unwrap().entries.values().cloned().collect::<Vec<_>>();

        if !values.is_empty() {
            let v = values.into_iter().map(|entry| entry.data).collect::<Vec<T>>();
            Ok(Some(v))
        } else {
            Ok(None)
        }
    }

    fn len(&mut self) -> anyhow::Result<usize>
    {
        match self.db.state.lock().unwrap().entries.len() {
            len if len > 0 => Ok(len),
            _ => Ok(0),
        }
    }

    fn purge(&mut self) -> anyhow::Result<()>
    {
        match self.db.purge() {
            Ok(_) => Ok(()),
            Err(e) => Err(e),
        }
    }

    fn get_many(&mut self, keys: &[&str]) -> anyhow::Result<Option<Vec<T>>>
    {
        let mut values = Vec::new();

        for key in keys {
            if let Ok(Some(v)) = self.db.get(key.to_string()) {
                values.push(v);
            }
        }

        if !values.is_empty() {
            Ok(Some(values))
        } else {
            Ok(None)
        }
    }

    fn set_many(&mut self, keys: &[&str], values: &[T]) -> anyhow::Result<()>
    {
        for (key, value) in keys.iter().zip(values.iter()) {
            self.db.set(key, value.clone(), None)?;
        }

        Ok(())
    }

    fn delete_many(&mut self, keys: &[&str]) -> anyhow::Result<()>
    {
        for key in keys {
            self.db.delete(key)?;
        }

        Ok(())
    }

    fn update_many(&mut self, keys: &[&str], values: &[T], upsert: Option<bool>) -> anyhow::Result<()>
    {
        for (key, value) in keys.iter().zip(values.iter()) {
            self.db.update(key, value.clone(), None, upsert)?;
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests
{
    use tempfile::tempdir;

    use super::*;
    use crate::types::HashSet;

    #[test]
    fn test_quick_client_set_get()
    {
        let tmp_dir = tempdir().expect("Failed to create tempdir");
        let tmp_file = tmp_dir.path().join("test.qkv").to_str().unwrap().to_string();

        let config = ClientConfig {
            path: Some(tmp_file),
            log: None,
            log_level: None,
            default_ttl: None,
        };
        let mut client = QuickClient::<String>::new(config);

        let key = "test_key";
        let value = "test_value".to_string();

        client.set(key, value.clone()).unwrap();
        let retrieved_value = client.get(key).unwrap().unwrap();

        assert_eq!(retrieved_value, value);
    }

    #[test]
    fn test_quick_client_delete()
    {
        let tmp_dir = tempdir().expect("Failed to create tempdir");
        let tmp_file = tmp_dir.path().join("test.qkv").to_str().unwrap().to_string();

        let config = ClientConfig {
            path: Some(tmp_file),
            log: None,
            log_level: None,
            default_ttl: None,
        };
        let mut client = QuickClient::<String>::new(config);

        let key = "test_key";
        let value = "test_value".to_string();

        client.set(key, value.clone()).unwrap();
        client.delete(key).unwrap();
        let retrieved_value = client.get(key).unwrap();

        assert!(retrieved_value.is_none());
    }

    #[test]
    fn test_quick_client_set_many_get_many()
    {
        let tmp_dir = tempdir().expect("Failed to create tempdir");
        let tmp_file = tmp_dir.path().join("test.qkv").to_str().unwrap().to_string();

        let config = ClientConfig {
            path: Some(tmp_file),
            log: None,
            log_level: None,
            default_ttl: None,
        };
        let mut client = QuickClient::<String>::new(config);

        let keys = vec!["key1", "key2", "key3"];
        let values = vec!["value1", "value2", "value3"]
            .iter()
            .map(|&s| s.to_string())
            .collect::<Vec<String>>();

        client.set_many(&keys, &values).unwrap();
        let retrieved_values = client.get_many(&keys).unwrap().unwrap();

        assert_eq!(retrieved_values, values);
    }

    #[test]
    fn test_quick_client_exists()
    {
        let tmp_dir = tempdir().expect("Failed to create tempdir");
        let tmp_file = tmp_dir.path().join("test.qkv").to_str().unwrap().to_string();

        let config = ClientConfig {
            path: Some(tmp_file),
            log: None,
            log_level: None,
            default_ttl: None,
        };
        let mut client = QuickClient::<String>::new(config);

        let key = "test_key";
        let value = "test_value".to_string();

        // Key doesn't exist yet
        assert_eq!(client.exists(key).unwrap(), false);

        // Set the key
        client.set(key, value.clone()).unwrap();

        // Key should now exist
        assert_eq!(client.exists(key).unwrap(), true);
    }

    #[test]
    fn test_quick_client_keys()
    {
        let tmp_dir = tempdir().expect("Failed to create tempdir");
        let tmp_file = tmp_dir.path().join("test.qkv").to_str().unwrap().to_string();

        let config = ClientConfig {
            path: Some(tmp_file),
            log: None,
            log_level: None,
            default_ttl: None,
        };
        let mut client = QuickClient::<String>::new(config);

        let keys = vec!["key1", "key2", "key3"];
        let values = vec!["value1", "value2", "value3"]
            .iter()
            .map(|&s| s.to_string())
            .collect::<Vec<String>>();

        client.set_many(&keys, &values).unwrap();

        let retrieved_keys = client.keys().unwrap().unwrap().into_iter().collect::<HashSet<_>>();
        let expected_keys: HashSet<_> = keys.iter().map(|&s| s.to_string()).collect();

        assert_eq!(retrieved_keys, expected_keys);
    }

    #[test]
    fn test_quick_client_values()
    {
        let tmp_dir = tempdir().expect("Failed to create tempdir");
        let tmp_file = tmp_dir.path().join("test.qkv").to_str().unwrap().to_string();

        let config = ClientConfig {
            path: Some(tmp_file),
            log: None,
            log_level: None,
            default_ttl: None,
        };
        let mut client = QuickClient::<String>::new(config);

        let keys = vec!["key1", "key2", "key3"];
        let values = vec!["value1", "value2", "value3"]
            .iter()
            .map(|&s| s.to_string())
            .collect::<Vec<String>>();

        client.set_many(&keys, &values).unwrap();

        let retrieved_values = client.values().unwrap().unwrap().into_iter().collect::<HashSet<_>>();
        let expected_values: HashSet<_> = values.iter().map(|s| s.to_string()).collect();

        assert_eq!(retrieved_values, expected_values);
    }

    #[test]
    fn test_quick_client_len()
    {
        let tmp_dir = tempdir().expect("Failed to create tempdir");
        let tmp_file = tmp_dir.path().join("test.qkv").to_str().unwrap().to_string();

        let config = ClientConfig {
            path: Some(tmp_file),
            log: None,
            log_level: None,
            default_ttl: None,
        };
        let mut client = QuickClient::<String>::new(config);

        let keys = vec!["key1", "key2", "key3"];
        let values = vec!["value1", "value2", "value3"]
            .iter()
            .map(|&s| s.to_string())
            .collect::<Vec<String>>();

        client.set_many(&keys, &values).unwrap();

        let length = client.len().unwrap();
        assert_eq!(length, 3);
    }

    #[test]
    fn test_quick_client_purge()
    {
        let tmp_dir = tempdir().expect("Failed to create tempdir");
        let tmp_file = tmp_dir.path().join("test.qkv").to_str().unwrap().to_string();

        let config = ClientConfig {
            path: Some(tmp_file),
            log: None,
            log_level: None,
            default_ttl: None,
        };
        let mut client = QuickClient::<String>::new(config);

        let key = "test_key";
        let value = "test_value".to_string();

        client.set(key, value.clone()).unwrap();
        client.purge().unwrap();

        assert_eq!(client.len().unwrap(), 0);
    }

    #[test]
    fn test_quick_client_update_many()
    {
        let config = ClientConfig {
            path: Some("test_db".to_string()),
            log: None,
            log_level: None,
            default_ttl: None,
        };

        let mut client = QuickClient::<String>::new(config);

        let keys = vec!["key1", "key2", "key3"];
        let values = vec!["value1", "value2", "value3"]
            .iter()
            .map(|&s| s.to_string())
            .collect::<Vec<String>>();

        client.set_many(&keys, &values).unwrap();

        let new_values = vec!["new_value1", "new_value2", "new_value3"]
            .iter()
            .map(|&s| s.to_string())
            .collect::<Vec<String>>();

        client.update_many(&keys, &new_values, None).unwrap();

        let retrieved_values = client.values().unwrap().unwrap();

        // Sort the retrieved and new values for comparison
        let mut sorted_retrieved_values = retrieved_values.clone();
        let mut sorted_new_values = new_values.clone();
        sorted_retrieved_values.sort();
        sorted_new_values.sort();

        assert_eq!(sorted_retrieved_values, sorted_new_values);
    }

    #[test]
    fn test_quick_client_delete_many()
    {
        let tmp_dir = tempdir().expect("Failed to create tempdir");
        let tmp_file = tmp_dir.path().join("test.qkv").to_str().unwrap().to_string();

        let config = ClientConfig {
            path: Some(tmp_file),
            log: None,
            log_level: None,
            default_ttl: None,
        };
        let mut client = QuickClient::<String>::new(config);

        let keys = vec!["key1", "key2", "key3"];
        let values = vec!["value1", "value2", "value3"]
            .iter()
            .map(|&s| s.to_string())
            .collect::<Vec<String>>();

        client.set_many(&keys, &values).unwrap();

        let keys_to_delete = vec!["key1", "key2"];

        client.delete_many(&keys_to_delete).unwrap();

        let remaining_keys = client.keys().unwrap().unwrap();
        assert_eq!(remaining_keys, vec!["key3"]);
    }
}
