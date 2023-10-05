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
use quick_kv::{QuickClient, Value};

fn main() {
    let mut client = QuickClient::new(None).unwrap();

    client
        .set("hello", Value::String("hello world!".to_string()))
        .unwrap();

    let result = match client.get::<Value>("hello").unwrap().unwrap() {
        Value::String(s) => s,
        _ => panic!("Error getting value"),
    };

    assert_eq!(result, String::from("hello world!"));
}
```
