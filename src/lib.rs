//! QuickKV is a simple key-value database written in Rust Its goal is to allow thread safe access to a database file with minimal
//! overhead It is not meant to be a replacement for a full fledged database, but rather a simple way to store data.
//!
//! # Features
//!
//! - Thread safe
//! - Simplistic API
//! - Minimal overhead
//! - Supports any type that implements `Serialize` and `Deserialize` from the serde crate
//!
//! # Installation
//!
//! ```shell
//! cargo add quick-kv
//! ```
//!
//! # Why use QuickKV?
//! QuickKV is meant to be used in situations where you need to store data, but don’t want to deal with the overhead of a full
//! fledged database. It is also useful for storing data that does’t need to be accessed very often, but still needs to be stored.
//! QuickKV also has the benefit of storing data in a file as binary making it faster than other file formats for data storage.
//!
//! # Working with Data in QuickKV
//!
//! ## Value
//!
//! When working with data using QuickKV, you will mostly be wrapping it in a [Value](https://docs.rs/quick-kv/latest/quick_kv/enum.Value.html) enum.
//! This Enum is used to tell the encoder what type of data you are storing. It also allows you to store multiple types of data in the same key.
//!
//! ## TypedValue
//!
//! The [TypedValue](https://docs.rs/quick-kv/latest/quick_kv/enum.TypedValue.html) enum is used to store data of types `Vec<T>`, `HashMap<String, V>`, and `Option<T>`.
//! These datatype need generic parameters to be typesafe and allow smart intellisense, so they are in there own enum.
//!
//! ## Into methods
//!
//! Both `Value` and `TypedValue` support helper methods for converting a value into a raw type or converting a raw type into a `Value` or `TypedValue`.
//!
//! This is usefull because when you wrap data inside of Value::String(), the compiler things its only an Enum of any value and not a String. We can fix this by:
//! ```rust
//! use quick_kv::{IntoValue, Value};
//!
//! Value::String("i am a real string!".to_string()).into_string();
//! ```
//!
//! Under the hood, into simply checks if the Value matches the type you are trying to convert it to and then unwraps it. This means that if you try to convert a Value::Int(5) into a String, it will panic
//! and crash your program. Right now, I think crashing is good as this is a critical mistake that should be noticed if you are using into.
//!
//! ## Using Operations
//! ```rust
//! use quick_kv::{QuickClient, IntoValue, Value, TypedValue, IntoTypedValue};
//!
//! let mut client = QuickClient::new(None).unwrap();
//!
//! // Set a i32 into the database.
//! client.set("i32", Value::I32(5).into_i32()).unwrap();
//!
//! // Get the i32 from the database and cast the type to the get function.
//! let our_i32 = client.get::<i32>("i32").unwrap().unwrap();
//!
//! assert_eq!(our_i32, 5);
//!
//! client.delete::<i32>("i32").unwrap();
//!
//! let mut list_of_people = vec!["Ray".to_string(), "Noa".to_string(), "Kian".to_string()];
//!
//! client.set("people", TypedValue::<String>::Vec(list_of_people.clone())).unwrap();
//!
//! let our_people = client.get::<TypedValue<String>>("people").unwrap().unwrap().into_vec();
//!
//! assert_eq!(our_people.len(), list_of_people.len());
//!
//! list_of_people.push("John".to_string());
//!
//! client.update("people", TypedValue::<String>::Vec(list_of_people.clone())).unwrap();
//!
//! let our_people = client.get::<TypedValue<String>>("people").unwrap().unwrap().into_vec();
//!
//! assert_eq!(our_people.len(), list_of_people.len());
//!```
//! Here is a general example of how to use QuickKV. A few important things to note:
//!
//! - When adding data into `Vectors/Hashmaps` or into set directly make sure to call `.into_<type>` to make sure
//! the right data is being saved into the database. Keep in mind, QuickKV is a binary based database, so it
//! needs the right information to work.
//! ```rust
//! use quick_kv::{QuickClient, IntoValue, Value};
//!
//! let mut client = QuickClient::new(None).unwrap();
//!
//! client.set("i32", Value::I32(5).into_i32()).unwrap();
//! // these 2 lines are different, and the same data is not saved in the db.
//! client.set("i32", Value::I32(5)).unwrap();
//!```
//!
//! - `into_vec` and `into_hashmap` are helper methods that allow you to convert a `TypedValue` into a `Vec<T>` or `HashMap<String, T>`.
//! If you dont use this (not required) then the get method will only return `TypeValue<T>`. This is the same for `Value`, but you can use `into_<type>` to convert it.

mod client;
mod test;
mod types;

pub use client::QuickClient;
pub use types::{
    BinaryKv, IntoTypedValue, IntoValue, RawIntoTypedValue, RawIntoValue, TypedValue, Value,
};
