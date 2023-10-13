//! QuickKV is a simple, fast, and easy to use key-value store.
//!
//! # Features
//!
//! - Simple API
//! - Built-in Logging
//! - I/O Caching
//! - CLI tool (Beta)
//!
//! ## Useful Links
//!
//! [Documentation] | [Crates.io] | [Github]
//!
//! ## Examples
//!
//! Examples can be found in the [examples] directory.
//!
//! [examples]: https://github.com/ThatGuyJamal/quick-kv/tree/master/examples
//!
//! [Documentation]: https://docs.rs/quick-kv
//! [Crates.io]: https://crates.io/crates/quick-kv
//! [Github]: https://github.com/ThatGuyJamal/quick-kv

#![allow(clippy::len_without_is_empty)]
#![allow(ambiguous_glob_reexports)]

pub mod clients;
pub mod prelude;

mod db;
mod types;
mod utils;
