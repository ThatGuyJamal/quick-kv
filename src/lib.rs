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
//! # Examples
//!
//! Coming soon...

mod test;
mod client;
mod types;

pub use client::QuickClient;
pub use types::{Value, BinaryKv, TypedValue, RawIntoValue, RawIntoTypedValue};