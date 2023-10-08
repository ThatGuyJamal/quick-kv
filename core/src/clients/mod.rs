use std::fmt::Debug;
use std::hash::Hash;
use std::time::Instant;

use serde::de::DeserializeOwned;
use serde::Serialize;

use crate::db::config::DatabaseConfiguration;

mod memory;

pub(crate) trait Client<T>
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
    /// use quick_kv::prelude::*;
    ///
    /// let mut client = QuickClient::<String>::new(None);
    /// ```
    /// *With custom configuration:*
    /// ```rust
    /// use quick_kv::prelude::*;
    ///
    /// let config = DatabaseConfiguration::new("db.qkv", RunTimeType::Disk, Some(true), Some(LevelFilter::Debug), None)
    ///
    /// let mut client = QuickClient::<String>::new(Some(config));
    /// ```
    fn new(&mut self, config: Option<DatabaseConfiguration>) -> Self;

    /// Get the value associated with a key.
    ///
    /// Returns `None` if the key does not exist. This could be caused by
    /// the key never being assigned a value, or the key expiring.
    ///
    /// # Examples
    /// ```rust
    /// use quick_kv::prelude::*;
    ///
    /// let mut client = QuickClient::<String>::new(None);
    ///
    /// let result = client.get("some-key")?;
    /// ```
    /// Do something with the result. After Consuming the result, you
    /// must handle the `Option<T>` that is returned.
    fn get(&mut self, key: &str) -> anyhow::Result<T>;
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
    /// let mut client = QuickClient::<String>::new(None);
    ///
    /// client.set("some-key", "some-value", None)?;
    /// ```
    fn set(&mut self, key: &str, value: T, ttl: Option<Instant>) -> anyhow::Result<()>;
    fn delete(&mut self, key: &str) -> anyhow::Result<()>;
    fn update(&mut self, key: &str, value: T, ttl: Option<Instant>) -> anyhow::Result<()>;

    fn exists(&mut self, key: &str) -> anyhow::Result<bool>;
    fn keys(&mut self) -> anyhow::Result<Vec<String>>;
    fn values(&mut self) -> anyhow::Result<Vec<T>>;
    fn len(&mut self) -> anyhow::Result<usize>;
    fn clear(&mut self) -> anyhow::Result<()>;

    fn get_many(&mut self, keys: &[&str]) -> anyhow::Result<Vec<T>>;
    fn set_many(&mut self, keys: &[&str], values: &[T], ttls: Option<Vec<Instant>>) -> anyhow::Result<()>;
    fn delete_many(&mut self, keys: &[&str]) -> anyhow::Result<()>;
    fn update_many(&mut self, keys: &[&str], values: &[T]) -> anyhow::Result<()>;
}
