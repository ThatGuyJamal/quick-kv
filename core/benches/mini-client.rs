use std::collections::HashMap;

use criterion::{black_box, criterion_group, criterion_main, Criterion};
use quick_kv::prelude::*;

fn client_mini(c: &mut Criterion)
{
    c.bench_function("client_mini", |b| {
        b.iter(|| {
            let mut client = QuickClientMini::new(None).unwrap();

            let mut map = HashMap::new();

            for i in 0..9 {
                map.insert(i.to_string(), i);
            }

            client.set("test-hash", TypedValue::<i32>::Hash(map.clone())).unwrap();

            let map_results = client.get::<TypedValue<i32>>("test-hash").unwrap().unwrap().into_hash();

            black_box(map_results);

            client.delete::<i32>("test-hash").unwrap();

            client.set("hello", Value::String("world".to_string()).into_string()).unwrap();

            black_box(client.get::<String>("hello").unwrap());

            client
                .update("hello", Value::String("world2".to_string()).into_string())
                .unwrap();

            black_box(client.get::<String>("hello").unwrap());

            client.delete::<String>("hello").unwrap();
        })
    });
}

criterion_group!(benches, client_mini);
criterion_main!(benches);
