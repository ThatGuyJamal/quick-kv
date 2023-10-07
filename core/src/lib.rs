//! QuickKV is a simple key-value database written in Rust Its goal is to allow thread safe access to a database file with minimal
//! overhead It is not meant to be a replacement for a full fledged database, but rather a simple way to store data.
//!
//! # Features
//!
//! - Simplistic API
//! - Serde Supported Data Types
//! - Thread safe
//!
//! ## Useful Links
//
// [Documentation] | [Crates.io] | [Github]
//!
//! # Installation
//!
//! ```shell
//! cargo add quick-kv --features full
//! ```
//! # Why use QuickKV?
//! QuickKV is a file persistent database that is meant to be used for simple or complex data structures. It is not meant to be a
//! replacement for a full fledged database, but rather a simple way to store data in a reliable, thread safe way.
//!
//! # Why not use QuickKV?
//! QuickKV is a local database. There are no plans for a server implementation. If you need a database that can be accessed by
//! multiple machines, you should look elsewhere. QuickKV also does not support async operations. It is designed to be used in a
//! synchronous environment.
//!
//! # Which Client Should I Use?
//!
//! QuickKV has two clients: [QuickClientMini] and [QuickClient].
//!
//! The [QuickClient] is disabled by default to reduce the dependency size. If you want to use the [QuickClient], you must enable
//! the `full` feature in your `Cargo.toml`.
//!
//!  ## QuickClientMini
//! [QuickClientMini] is the simplest client. It allows you to CRUD data of any type by leveraging per method generics. It is the
//! recommended client if you have very inconsistent data you want to store in the database or don't
//! need high performance read times.
//!
//! Pros:
//! - Minimal API
//! - Flexible Data-store
//! - Small Dependency Size
//!
//! Cons:
//! - No internal multi-threading
//! - Wrappers needed for type safety
//! - Unoptimized caching
//!
//! ## QuickClient
//! [QuickClient] is a client that is optimized for a specific schema and has multi-threading enabled by default. This client
//! is built for speed and is recommended if you want to take advanced of cached data and multi-threading.
//!
//! Pros:
//! - Uses multi-threading for internal operations
//! - Strict Data-store schema
//! - Simpler Type safety
//! - Optimized caching
//!
//! Cons:
//! - Single Schema per instance
//! - Simple Data-store type
//!
//! # Examples
//!
//! Examples can be found in the [examples] directory.
//!
//! [QuickClientMini]: client/mini/struct.QuickClientMini.html
//! [QuickClient]: client/core/struct.QuickClient.html
//! [QuickConfiguration]: struct.QuickConfiguration.html
//! [examples]: https://github.com/ThatGuyJamal/quick-kv/tree/master/core/examples
//!
//! [Documentation]: https://docs.rs/quick-kv
//! [Crates.io]: https://crates.io/crates/quick-kv
//! [Github]: https://github.com/ThatGuyJamal/quick-kv

pub mod client;
pub mod prelude;
pub mod types;
mod utils;
