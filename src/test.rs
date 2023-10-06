#[cfg(test)]
mod tests {
    use crate::prelude::*;
    use std::collections::HashMap;
    use tempfile::tempdir;

    #[test]
    fn test_set() {
        let tmp_dir = tempdir().expect("Failed to create tempdir");
        let tmp_file = tmp_dir.path().join("test.qkv");

        let mut client = QuickClient::new(Some(tmp_file)).unwrap();

        let value = String::from("Hello World!");
        client.set("hello", value).unwrap();
    }

    #[test]
    fn test_set_multiple_keys_with_same_name() {
        let tmp_dir = tempdir().expect("Failed to create tempdir");
        let tmp_file = tmp_dir.path().join("test.qkv");

        let mut client = QuickClient::new(Some(tmp_file.clone())).unwrap();

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
    fn test_get() {
        let tmp_dir = tempdir().expect("Failed to create tempdir");
        let tmp_file = tmp_dir.path().join("test.qkv");

        let mut client = QuickClient::new(Some(tmp_file)).unwrap();

        let value = String::from("Hello World!");
        client.set("hello2", value.clone()).unwrap();

        let result = client.get::<String>("hello2").unwrap();
        assert_eq!(result, Some(value));
    }

    #[test]
    fn test_get_not_found() {
        let tmp_dir = tempdir().expect("Failed to create tempdir");
        let tmp_file = tmp_dir.path().join("test.qkv");

        let mut client = QuickClient::new(Some(tmp_file)).unwrap();

        let value = String::from("Hello World!");
        client.set("hello3", value).unwrap();

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

        client.set("hello5", value.clone()).unwrap();
        client.set("hello6", value.clone()).unwrap();

        let result = client.get::<String>("hello5").unwrap();
        assert_eq!(result, Some(value)); // Clone the value to compare it
    }

    #[test]
    fn test_delete() {
        let tmp_dir = tempdir().expect("Failed to create tempdir");
        let tmp_file = tmp_dir.path().join("test.qkv");

        let mut client = QuickClient::new(Some(tmp_file)).unwrap();
        let value = String::from("Hello World!");

        client.set("hello7", value.clone()).unwrap();
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

        for i in 0..9 {
            v.push(i);
        }

        client
            .set("vec", TypedValue::<i32>::Vec(v.clone()))
            .unwrap();

        let result = client
            .get::<TypedValue<i32>>("vec")
            .unwrap()
            .unwrap()
            .into_vec();

        for i in 0..9 {
            assert_eq!(result[i], v[i]);
        }

        assert_eq!(result.len(), v.len());
    }

    #[test]
    fn test_hashmap_injection() {
        let tmp_dir = tempdir().expect("Failed to create tempdir");
        let tmp_file = tmp_dir.path().join("test.qkv");

        let mut client = QuickClient::new(Some(tmp_file.clone())).unwrap();

        let mut map = HashMap::new();

        for i in 0..4 {
            map.insert(i.to_string(), i);
        }

        client
            .set("map", TypedValue::<i32>::Hash(map.clone()))
            .unwrap();

        let result = client
            .get::<TypedValue<i32>>("map")
            .unwrap()
            .unwrap()
            .into_hash();

        assert_eq!(result.len(), map.len());
    }
}

#[cfg(feature = "full")]
#[cfg(test)]
mod feature_tests {
    use std::fs;
    use crate::prelude::*;
    use tempfile::tempdir;

    #[test]
    fn test_client_new() -> std::io::Result<()> {
        let tmp_dir = tempdir().expect("Failed to create tempdir");
        let tmp_file = tmp_dir.path().join("test.qkv");

        match QuickSchemaClient::<String>::new(Some(tmp_file)) {
            Ok(_) => Ok(()),
            Err(e) => {
                Err(std::io::Error::new(
                    std::io::ErrorKind::Other,
                    format!("Failed to create QuickSchemaClient: {}", e),
                ))
            }
        }
    }

    #[test]
    fn test_get_and_set() -> std::io::Result<()> {
        let tmp_dir = tempdir().expect("Failed to create tempdir");
        let tmp_file = tmp_dir.path().join("test.qkv");

        let mut client = QuickSchemaClient::<String>::new(Some(tmp_file.clone()))?;

        client.set("hello", String::from("Hello World!"))?;

        let result = client.get("hello")?;

        assert_eq!(result, Some(String::from("Hello World!")));

        Ok(())
    }

    #[test]
    fn test_clear() -> std::io::Result<()> {
        let tmp_dir = tempdir().expect("Failed to create tempdir");
        let tmp_file = tmp_dir.path().join("test.qkv");

        let mut client = QuickSchemaClient::<i32>::new(Some(tmp_file.clone())).unwrap();

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
    fn test_get_all() -> std::io::Result<()> {
        let tmp_dir = tempdir().expect("Failed to create tempdir");
        let tmp_file = tmp_dir.path().join("test.qkv");

        let mut client = QuickSchemaClient::<i32>::new(Some(tmp_file.clone())).unwrap();

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
    fn test_get_many() -> std::io::Result<()> {
        let tmp_dir = tempdir().expect("Failed to create tempdir");
        let tmp_file = tmp_dir.path().join("test.qkv");

        let mut client = QuickSchemaClient::<i32>::new(Some(tmp_file.clone())).unwrap();

        // Add some data to the cache
        client.set("key1", 42)?;
        client.set("key2", 77)?;

        // Get specific keys from the cache
        let keys_to_get = vec!["key1".to_string(), "key2".to_string()];
        let values = client.get_many(keys_to_get)?;

        // Check if values are retrieved correctly
        assert_eq!(values.len(), 2);
        assert!(values.contains(&42));
        assert!(values.contains(&77));

        Ok(())
    }

    #[test]
    fn test_set_many() -> std::io::Result<()> {
        let tmp_dir = tempdir().expect("Failed to create tempdir");
        let tmp_file = tmp_dir.path().join("test.qkv");

        let mut client = QuickSchemaClient::<i32>::new(Some(tmp_file.clone())).unwrap();

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
    fn test_delete_many() -> std::io::Result<()> {
        let tmp_dir = tempdir().expect("Failed to create tempdir");
        let tmp_file = tmp_dir.path().join("test.qkv");

        let mut client = QuickSchemaClient::<i32>::new(Some(tmp_file.clone())).unwrap();

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
}
