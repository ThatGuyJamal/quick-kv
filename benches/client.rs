use std::path::PathBuf;

use criterion::{black_box, criterion_group, criterion_main, Criterion};
use quick_kv::prelude::*;
use serde::{Deserialize, Serialize};

fn client(c: &mut Criterion)
{
    c.bench_function("client", |b| {
        b.iter(|| {
            let config = QuickConfiguration {
                path: Some(PathBuf::from("db.qkv")),
                logs: false, // Disable logging for the benchmark
                log_level: None,
            };

            #[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
            struct Books
            {
                title: String,
                description: String,
                pages: u32,
            }

            let mut client = QuickClient::<Books>::new(Some(config)).unwrap();

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

            black_box(results);
        })
    });
}

criterion_group!(benches, client);
criterion_main!(benches);
