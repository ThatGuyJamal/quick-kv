use serde::Serialize;
use std::fmt::Debug;
use serde::de::DeserializeOwned;
use std::hash::Hash;
use std::time::Instant;
use crate::clients::{BaseClient, ClientConfig};
use crate::db::config::DatabaseConfiguration;

use crate::db::Database;
use crate::db::runtime::{RunTime, RuntTimeType};

#[derive(Debug)]
pub struct QuickClient<T>
where
    T: Serialize + DeserializeOwned + Debug + Eq + PartialEq + Hash + Send + Sync + Clone + 'static,
{
    db: Database<T>,
}

impl<T> BaseClient<T> for QuickClient<T>
where
    T: Serialize + DeserializeOwned + Debug + Eq + PartialEq + Hash + Send + Sync + Clone + 'static,
{
    fn new(config: ClientConfig) -> Self {

        let _config = DatabaseConfiguration::new(config.path, Some(RunTime::new(RuntTimeType::Disk)), config.log, config.log_level, config.default_ttl).unwrap();

        let db = Database::new(_config).unwrap();

        Self {
            db
        }
    }

    fn get(&mut self, key: &str) -> anyhow::Result<Option<T>> {
        match self.db.get(key.to_string()) {
            Ok(value) => Ok(value),
            Err(e) => Err(e)
        }
    }

    fn set(&mut self, key: &str, value: T, ttl: Option<Instant>) -> anyhow::Result<()> {
        todo!()
    }

    fn update(&mut self, key: &str, value: T, ttl: Option<Instant>, upsert: Option<bool>) -> anyhow::Result<()> {
        todo!()
    }

    fn delete(&mut self, key: &str) -> anyhow::Result<()> {
        todo!()
    }

    fn exists(&mut self, key: &str) -> anyhow::Result<bool> {
        todo!()
    }

    fn keys(&mut self) -> anyhow::Result<Option<Vec<String>>> {
        todo!()
    }

    fn values(&mut self) -> anyhow::Result<Option<Vec<T>>> {
        todo!()
    }

    fn len(&mut self) -> anyhow::Result<usize> {
        todo!()
    }

    fn clear(&mut self) -> anyhow::Result<()> {
        todo!()
    }

    fn get_many(&mut self, keys: &[&str]) -> anyhow::Result<Option<Vec<T>>> {
        todo!()
    }

    fn set_many(&mut self, keys: &[&str], values: &[T], ttls: Option<Vec<Instant>>) -> anyhow::Result<()> {
        todo!()
    }

    fn delete_many(&mut self, keys: &[&str]) -> anyhow::Result<()> {
        todo!()
    }

    fn update_many(&mut self, keys: &[&str], values: &[T]) -> anyhow::Result<()> {
        todo!()
    }
}