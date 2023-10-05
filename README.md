# Quick-KV

A Fast Key Value Database in rust.

## Features

- [x] Binary Based Data
- [x] Multiple Data Type Support
- [ ] Multiple Database Management (todo)

## Installation

```bash
cargo add quick-kv
```

## Usage

```rust
use std::collections::HashMap;
use quick_kv::*;

fn main() {
    let mut client = QuickClient::new(None).unwrap();

    let mut map = HashMap::new();

    for i in 0..49 {
        map.insert(i.to_string(), Value::String(i.to_string()).into_string());
    }

    client
        .set("test-hash", TypedValue::<String>::Hash(map.clone()))
        .unwrap();

    let map_results = client
        .get::<TypedValue<String>>("test-hash")
        .unwrap()
        .unwrap()
        .into_hash();

    for (key, value) in map_results.iter() {
        assert_eq!(map.get(key).unwrap(), value);
    }

    assert!(map_results.len() == map.len());

    println!("All tests passed!")
}

```