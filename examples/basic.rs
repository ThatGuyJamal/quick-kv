use std::collections::HashMap;

use quick_kv::prelude::*;

fn main()
{
    // Create a new client with the default options
    let mut client = QuickClient::new(None).unwrap();

    let mut map = HashMap::new();

    // Add some data to the map
    for i in 0..9 {
        map.insert(i.to_string(), i);
    }

    // Save the map to the database
    // We need to tell the database what our data is, so it must be wrapped in a TypedValue.
    // In this instance we use TypedValue::Hash to tell the database that the data is a HashMap of type i32
    client.set("test-hash", TypedValue::<i32>::Hash(map.clone())).unwrap();

    // Get the data from the database
    // We again have to pass TypeValue to the get method.
    // We can call a helper called into_hash to convert the data into a HashMap.
    // if this is not used, Your type returned will be TypedValue<i32> which is not very useful.
    let map_results = client.get::<TypedValue<i32>>("test-hash").unwrap().unwrap().into_hash();

    for (key, value) in map_results.iter() {
        println!("{}: {}", key, value)
    }

    assert_eq!(map, map_results);

    // Delete the data from the database
    client.delete::<i32>("test-hash").unwrap();

    assert!(client.get::<TypedValue<i32>>("test-hash").unwrap().is_none());

    // Add some new data to the database
    // As you can see, the QuickClient can consume many different types of data in the same database instance.
    client.set("hello", Value::String("world".to_string()).into_string()).unwrap();

    assert_eq!(client.get::<String>("hello").unwrap().unwrap(), "world".to_string());

    // Updating an existing key is the same as setting a new key.
    client
        .update("hello", Value::String("world2".to_string()).into_string())
        .unwrap();

    assert_eq!(client.get::<String>("hello").unwrap().unwrap(), "world2".to_string());

    client.delete::<String>("hello").unwrap();

    assert!(client.get::<String>("hello").unwrap().is_none());
}
