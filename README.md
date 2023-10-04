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
use quick_kv::QuickClient;

fn main() {
    let mut client = QuickClient::new(None).unwrap();

    client.set::<String>("key", "value".to_string());
    
    let result = client.get::<String>("key").unwrap();

    println!("{}", result);
}
```
