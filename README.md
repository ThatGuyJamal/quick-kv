# Quick-KV

A Fast Key Value Database in rust.

## Features

- [ ] Binary Based Data
- [ ] Multiple Database Management
- [ ]

## Installation

```bash
cargo add quick-kv
```

## Usage

```rust
use quick_kv::QuickClient;

fn main() {
    let client = QuickClient::new(None)

    let result = client.get("key").unwrap();

    println!("{}", result);
}
```
