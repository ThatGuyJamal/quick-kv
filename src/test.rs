#[cfg(test)]
mod tests {
    use crate::prelude::*;
    use serde::{Deserialize, Serialize};
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
    use crate::prelude::*;
    use tempfile::tempdir;

    #[test]
    fn test_client_new() {
        let tmp_dir = tempdir().expect("Failed to create tempdir");
        let tmp_file = tmp_dir.path().join("test.qkv");

        match QuickSchemaClient::<String>::new(Some(tmp_file)) {
            Ok(_) => {}
            Err(e) => {
                panic!("Error creating client: {:?}", e);
            }
        }
    }
}
