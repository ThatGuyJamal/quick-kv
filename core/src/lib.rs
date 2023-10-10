#![allow(dead_code)] // todo: remove this
#![allow(unused_imports)] // todo: remove this
#![allow(unused_variables)] // todo: remove this

//! QuickKV is a offline key-value database designed to be thread safe, fast, and easy to use.
//!
//! # Features
//!
//! - todo
//!
//! ## Useful Links
//
// [Documentation] | [Crates.io] | [Github]
//!
//! # Why use QuickKV?
//!
//! todo
//!
//! Examples can be found in the [examples] directory.
//!
//! [examples]: https://github.com/ThatGuyJamal/quick-kv/tree/master/core/examples
//!
//! [Documentation]: https://docs.rs/quick-kv
//! [Crates.io]: https://crates.io/crates/quick-kv
//! [Github]: https://github.com/ThatGuyJamal/quick-kv

pub mod prelude;

mod clients;
mod db;
mod types;