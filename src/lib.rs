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
//!  ## QuickClientMini
//! [QuickClientMini] is the simplest client. It allows you to CRUD data of any type by leveraging per method generics. It is the
//! recommended client if you have very inconsistent data you want to store in the database or don't
//! need high performance read times.
//!
//! Pros:
//! - Can store any type of data
//! - No need to define a schema
//! - Simple API
//! - Database can store multiple types of data
//!
//! Cons:
//! - Does not use multi-threading for internal operations
//! - No caching
//! - No strict schema
//! - Lots of type conversions to have type safety
//! - No Complex APIs for advanced operations
//!
//! ## QuickClient
//! [QuickClient] is a client that is optimized for a specific schema and has multi-threading enabled by default. This client
//! is built for speed and is recommended if you want to take advanced of cached data and multi-threading.
//!
//! Pros:
//! - Uses multi-threading for internal operations
//! - Internal caching
//! - Strict schema
//! - Type safety
//!
//! Cons:
//! - Must define a schema
//! - Must use a specific type for all data
//! - Database can only store one type of data
//! - More complex API
//!
//! Both clients have there own pros and cons. It is up to you to decide which client is best for your use case.
//!
//! # Examples
//!
//! Examples can be found in the [examples] directory.
//!
//! [QuickClientMini]: client/mini/struct.QuickClientMini.html
//! [QuickClient]: client/core/struct.QuickClient.html
//! [QuickConfiguration]: struct.QuickConfiguration.html
//! [examples]: https://github.com/ThatGuyJamal/quick-kv/tree/master/examples
//!
//! [Documentation]: https://docs.rs/quick-kv
//! [Crates.io]: https://crates.io/crates/quick-kv
//! [Github]: https://github.com/ThatGuyJamal/quick-kv

pub mod client;
pub mod prelude;
pub mod types;
