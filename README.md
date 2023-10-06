# Quick-KV

A Fast Key Value Database in rust.

## Features

- [x] Binary Based Data-Store
- [x] Serde Supported Data Types
- [x] Thread Safe

## Installation

```bash
cargo add quick-kv --features full
```

## Documentation

For the most detailed documentation, please see the [docs.rs](https://docs.rs/quick-kv) page.

## Usage

*The examples below require the `full` feature to be enabled.*
```rust
use std::path::PathBuf;

use quick_kv::prelude::*;
use serde::{Deserialize, Serialize};

fn main()
{
    let config = QuickConfiguration {
        path: Some(PathBuf::from("db.qkv")),
        logs: true,
        log_level: Some(LevelFilter::Debug),
    };

    #[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
    struct Books
    {
        title: String,
        description: String,
        pages: u32,
    }

    let mut client = QuickSchemaClient::<Books>::new(Some(config)).unwrap();

    let books = vec![
        Books {
            title: "The Hobbit".to_string(),
            description: "A book about a hobbit".to_string(),
            pages: 300,
        },
        Books {
            title: "The Lord of the Rings".to_string(),
            description: "A book about a ring".to_string(),
            pages: 500,
        },
        Books {
            title: "The Evil Kind".to_string(),
            description: "A book about an evil king".to_string(),
            pages: 200,
        },
    ];

    let mut data = Vec::new();

    for i in 0..books.len() {
        data.push(BinaryKv {
            key: i.to_string(),
            value: books[i].clone(),
        });
    }

    client.set_many(data).unwrap();

    let results = client
        .get_many(vec!["0".to_string(), "1".to_string(), "2".to_string()])
        .unwrap();

    for i in 0..results.len() {
        println!("Found book: {:?}", results[i].value.title)
    }

    assert_eq!(results.len(), books.len());
}
```