use std::fmt::Debug;
use std::hash::Hash;
use std::time::{Duration, Instant};

use log::LevelFilter;
use serde::de::DeserializeOwned;
use serde::Serialize;

use crate::db::config::DatabaseConfiguration;

pub mod memory;
pub mod normal;

#[derive(Debug, Clone)]
pub struct ClientConfig
{
    /// The path to the database file.
    ///
    /// Default: "db.qkv"
    pub path: Option<String>,
    /// If the database should log to stdout.
    ///
    /// Default: true
    pub log: Option<bool>,
    /// The log level to use for the database.
    ///
    /// Default: LevelFilter::Info
    pub log_level: Option<LevelFilter>,
    /// The default time-to-live for entries in the database.
    ///
    /// If enabled, all entries will have a ttl by default.
    /// If disabled (None), then you will have to manually set the ttl for each entry.
    ///
    /// Default: None
    pub default_ttl: Option<Duration>,
}

impl ClientConfig
{
    pub fn new(path:String, log: Option<bool>, log_level: Option<LevelFilter>) -> Self
    {
        Self {
            path: Some(path),
            log,
            log_level,
            default_ttl: None,
        }
    }
}

impl Default for ClientConfig
{
    fn default() -> Self
    {
        Self {
            path: "cli.qkv".to_string().into(),
            log: true.into(),
            log_level: LevelFilter::Info.into(),
            default_ttl: None,
        }
    }
}

pub trait BaseClient<T>
where
    T: Serialize + DeserializeOwned + Debug + Eq + PartialEq + Hash + Send + Sync,
{
    /// Creates a new instance of the client.
    ///
    /// `config` is the configuration for the database. If `None`, then the default configuration will be used.
    ///
    /// The client needs to know what type of data it will be storing, so it can properly serialize and deserialize it.
    /// You need to specify the type of data when creating a new client using the `client::<T>::new()` method.
    ///
    /// This type must implement the following traits:
    /// - `Serialize`
    /// - `DeserializeOwned`
    /// - `Debug`
    /// - `Eq`
    /// - `PartialEq`
    /// - `Hash`
    /// - `Send`
    /// - `Sync`
    ///
    /// # Examples
    /// *With default configuration:*
    /// ```rust
    /// ```
    fn new(config: ClientConfig) -> Self;

    /// Get the value associated with a key.
    ///
    /// Returns `None` if the key does not exist. This could be caused by
    /// the key never being assigned a value, or the key expiring.
    ///
    /// # Examples
    /// ```rust
    /// ```
    /// Do something with the result. After Consuming the result, you
    /// must handle the `Option<T>` that is returned.
    fn get(&mut self, key: &str) -> anyhow::Result<Option<T>>;
    /// Set the value associated with a key.
    ///
    /// If the key already exists, the database will attempt to overwrite the value.
    ///
    /// `key` to set the value for.
    ///
    /// `value` to set for the key.
    ///
    /// `ttl` the time-to-live for the key. If `None`, then the the key will not expire,
    /// unless `default_ttl` is set in the configuration. If `default_ttl` is set, then
    /// the key will expire after the default ttl. If ttl is set here, it will override
    /// the default ttl set in the configuration.
    ///
    /// # Examples
    /// ```rust
    /// ```
    fn set(&mut self, key: &str, value: T) -> anyhow::Result<()>;
    fn update(&mut self, key: &str, value: T, upsert: Option<bool>) -> anyhow::Result<()>;
    fn delete(&mut self, key: &str) -> anyhow::Result<()>;

    fn exists(&mut self, key: &str) -> anyhow::Result<bool>;
    fn keys(&mut self) -> anyhow::Result<Option<Vec<String>>>;
    fn values(&mut self) -> anyhow::Result<Option<Vec<T>>>;
    fn len(&mut self) -> anyhow::Result<usize>;
    fn purge(&mut self) -> anyhow::Result<()>;

    fn get_many(&mut self, keys: &[&str]) -> anyhow::Result<Option<Vec<T>>>;
    fn set_many(&mut self, keys: &[&str], values: &[T]) -> anyhow::Result<()>;
    fn delete_many(&mut self, keys: &[&str]) -> anyhow::Result<()>;
    fn update_many(&mut self, keys: &[&str], values: &[T], upsert: Option<bool>) -> anyhow::Result<()>;
}
