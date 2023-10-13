use std::fmt::Debug;
use std::hash::Hash;
use std::time::Duration;

use log::LevelFilter;
use serde::de::DeserializeOwned;
use serde::Serialize;

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
    pub fn new(path: String, log: Option<bool>, log_level: Option<LevelFilter>) -> Self
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
            path: "db.qkv".to_string().into(),
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
    /// - `Clone`
    ///
    /// # Examples
    /// *With default configuration:*
    /// ```rust
    /// use quick_kv::prelude::*;
    ///
    /// #[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
    /// struct Schema
    /// {
    ///     id: u64,
    /// };
    ///
    /// let mut client = QuickClient::<Schema>::new(ClientConfig::new(
    ///     "db.qkv".to_string(),
    ///     true.into(),
    ///     LevelFilter::Debug.into(),
    /// ));
    /// ```
    fn new(config: ClientConfig) -> Self;

    /// Get the value associated with a key.
    ///
    /// Returns `None` if the key does not exist. This could be caused by
    /// the key never being assigned a value, or the key expiring.
    ///
    /// # Examples
    /// ```rust
    /// use quick_kv::prelude::*;
    ///
    /// #[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
    /// struct Schema
    /// {
    ///     id: u64,
    /// }
    ///
    /// let mut client = QuickClient::<Schema>::new(ClientConfig::new(
    ///     "db.qkv".to_string(),
    ///     true.into(),
    ///     LevelFilter::Debug.into(),
    /// ));
    ///
    /// client.set("user_1", Schema { id: 10 }).unwrap();
    ///
    /// let user = client.get("user_1").unwrap();
    ///
    /// // do something with the user
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
    /// use quick_kv::prelude::*;
    ///
    /// #[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
    /// struct Schema
    /// {
    ///     id: u64,
    /// };
    ///
    /// let mut client = QuickClient::<Schema>::new(ClientConfig::new(
    ///     "db.qkv".to_string(),
    ///     true.into(),
    ///     LevelFilter::Debug.into(),
    /// ));
    ///
    /// client.set("user_1", Schema { id: 10 }).unwrap();
    /// ```
    fn set(&mut self, key: &str, value: T) -> anyhow::Result<()>;
    /// Update the value associated with a key.
    ///
    /// By default update will fail if the key does not exist. If you want to upsert the value, then
    /// you can set `upsert` to `true` using `true.into()` or `Some(true)`.
    ///
    /// `key` to update the value for.
    ///
    /// `value` to update for the key.
    ///
    /// `upsert` if the value should be upserted if the key does not exist.
    ///
    /// # Examples
    /// ```rust
    /// use quick_kv::prelude::*;
    ///
    /// #[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
    /// struct Schema
    /// {
    ///     id: u64,
    /// };
    ///
    /// let mut client = QuickClient::<Schema>::new(ClientConfig::new(
    ///     "db.qkv".to_string(),
    ///     true.into(),
    ///     LevelFilter::Debug.into(),
    /// ));
    ///
    /// client.update("user_1", Schema { id: 10 }, None).unwrap(); // fails
    /// client
    ///     .update("user_1", Schema { id: 20 }, true.into())
    ///     .unwrap(); // succeeds
    /// ```
    fn update(&mut self, key: &str, value: T, upsert: Option<bool>) -> anyhow::Result<()>;

    /// Delete the value associated with a key.
    ///
    /// `key` to delete the value for.
    ///
    /// # Examples
    /// ```rust
    /// use quick_kv::prelude::*;
    ///
    /// #[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
    /// struct Schema
    /// {
    ///     id: u64,
    /// };
    ///
    /// let mut client = QuickClient::<Schema>::new(ClientConfig::new(
    ///     "db.qkv".to_string(),
    ///     true.into(),
    ///     LevelFilter::Debug.into(),
    /// ));
    ///
    /// client.delete("user_1").unwrap();
    /// ```
    fn delete(&mut self, key: &str) -> anyhow::Result<()>;
    /// Check if a key exists in the database.
    ///
    /// `key` to check if it exists.
    ///
    /// # Examples
    /// ```rust
    /// use quick_kv::prelude::*;
    ///
    /// #[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
    /// struct Schema
    /// {
    ///     id: u64,
    /// };
    ///
    /// let mut client = QuickClient::<Schema>::new(ClientConfig::new(
    ///     "db.qkv".to_string(),
    ///     true.into(),
    ///     LevelFilter::Debug.into(),
    /// ));
    ///
    /// if client.exists("user_1").unwrap() {
    ///     // do something
    /// }
    /// ```
    fn exists(&mut self, key: &str) -> anyhow::Result<bool>;
    /// Get all keys in the database.
    ///
    /// Returns `None` if there are no keys in the database or a `Vec<String>` keys.
    ///
    /// # Examples
    /// ```rust
    /// use quick_kv::prelude::*;
    ///
    /// #[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
    /// struct Schema
    /// {
    ///     id: u64,
    /// };
    ///
    /// let mut client = QuickClient::<Schema>::new(ClientConfig::new(
    ///     "db.qkv".to_string(),
    ///     true.into(),
    ///     LevelFilter::Debug.into(),
    /// ));
    ///
    /// let all_keys = client.keys().unwrap();
    /// ```
    fn keys(&mut self) -> anyhow::Result<Option<Vec<String>>>;
    /// Get all values in the database.
    ///
    /// Returns `None` if there are no values in the database or a `Vec<T>` values.
    ///
    /// # Examples
    /// ```rust
    /// use quick_kv::prelude::*;
    ///
    /// #[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
    /// struct Schema
    /// {
    ///     id: u64,
    /// };
    ///
    /// let mut client = QuickClient::<Schema>::new(ClientConfig::new(
    ///     "db.qkv".to_string(),
    ///     true.into(),
    ///     LevelFilter::Debug.into(),
    /// ));
    ///
    /// let all_values = client.values().unwrap();
    /// ```
    fn values(&mut self) -> anyhow::Result<Option<Vec<T>>>;
    /// Get the number of keys in the database.
    ///
    /// Returns `0` if there are no keys in the database or the number of keys in the database.
    ///
    /// # Examples
    /// ```rust
    /// use quick_kv::prelude::*;
    ///
    /// #[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
    /// struct Schema
    /// {
    ///     id: u64,
    /// };
    ///
    /// let mut client = QuickClient::<Schema>::new(ClientConfig::new(
    ///     "db.qkv".to_string(),
    ///     true.into(),
    ///     LevelFilter::Debug.into(),
    /// ));
    ///
    /// let num_keys = client.len().unwrap();
    /// ```
    fn len(&mut self) -> anyhow::Result<usize>;
    /// Clears all keys and values from the database.
    /// # Examples
    /// ```rust
    /// use quick_kv::prelude::*;
    ///
    /// #[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
    /// struct Schema
    /// {
    ///     id: u64,
    /// };
    ///
    /// let mut client = QuickClient::<Schema>::new(ClientConfig::new(
    ///     "db.qkv".to_string(),
    ///     true.into(),
    ///     LevelFilter::Debug.into(),
    /// ));
    ///
    /// client.purge().unwrap();
    /// ```
    fn purge(&mut self) -> anyhow::Result<()>;
    /// Get multiple values associated with multiple keys.
    /// # Examples
    /// ```rust
    /// use quick_kv::prelude::*;
    ///
    /// #[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
    /// struct Schema
    /// {
    ///     id: u64,
    /// };
    ///
    /// let mut client = QuickClient::<Schema>::new(ClientConfig::new(
    ///     "db.qkv".to_string(),
    ///     true.into(),
    ///     LevelFilter::Debug.into(),
    /// ));
    ///
    /// let values = client.get_many(&["user_1", "user_2"]).unwrap();
    /// ```
    fn get_many(&mut self, keys: &[&str]) -> anyhow::Result<Option<Vec<T>>>;
    /// Set multiple values associated with multiple keys.
    ///
    /// # Examples
    /// ```rust
    /// use quick_kv::prelude::*;
    ///
    /// #[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
    /// struct Schema
    /// {
    ///     id: u64,
    /// };
    ///
    /// let mut client = QuickClient::<Schema>::new(ClientConfig::new(
    ///     "db.qkv".to_string(),
    ///     true.into(),
    ///     LevelFilter::Debug.into(),
    /// ));
    ///
    /// client
    ///     .set_many(
    ///         &["user_1", "user_2"],
    ///         &[Schema { id: 10 }, Schema { id: 20 }],
    ///     )
    ///     .unwrap();
    /// ```
    fn set_many(&mut self, keys: &[&str], values: &[T]) -> anyhow::Result<()>;
    /// Delete multiple values associated with multiple keys.
    ///
    /// # Examples
    /// ```rust
    /// use quick_kv::prelude::*;
    ///
    /// #[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
    /// struct Schema
    /// {
    ///     id: u64,
    /// };
    ///
    /// let mut client = QuickClient::<Schema>::new(ClientConfig::new(
    ///     "db.qkv".to_string(),
    ///     true.into(),
    ///     LevelFilter::Debug.into(),
    /// ));
    ///
    /// client.delete_many(&["user_1", "user_2"]).unwrap();
    /// ```
    fn delete_many(&mut self, keys: &[&str]) -> anyhow::Result<()>;
    /// Update multiple values associated with multiple keys.
    ///
    /// # Examples
    /// ```rust
    /// use quick_kv::prelude::*;
    ///
    /// #[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
    /// struct Schema {
    ///  id: u64,
    /// };
    ///
    /// let mut client = QuickClient::<Schema>::new(ClientConfig::new("db.qkv".to_string(), true.into(), LevelFilter::Debug.into()));
    ///
    /// client.update_many(&["user_1", "user_2"], &[Schema { id: 10 }, Schema { id: 20 }], true.into()).unwrap();
    fn update_many(&mut self, keys: &[&str], values: &[T], upsert: Option<bool>) -> anyhow::Result<()>;
}
