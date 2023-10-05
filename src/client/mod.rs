use crate::types::BinaryKv;
use serde::de::DeserializeOwned;
use serde::Serialize;
use std::fmt::Debug;
use std::io;
use std::path::PathBuf;

pub mod normal;

#[cfg(feature = "full")]
pub mod schema;

pub trait Client {
    fn new(path: Option<PathBuf>) -> io::Result<Self>
    where
        Self: Sized + Debug;

    fn get<T>(&mut self, key: &str) -> io::Result<Option<T>>
    where
        T: Serialize + DeserializeOwned + Clone + Debug;

    fn set<T>(&mut self, key: &str, value: T) -> io::Result<()>
    where
        T: Serialize + DeserializeOwned + Clone + Debug;

    fn delete<T>(&mut self, key: &str) -> io::Result<()>
    where
        T: Serialize + DeserializeOwned + Clone + Debug;

    fn update<T>(&mut self, key: &str, value: T) -> io::Result<()>
    where
        T: Serialize + DeserializeOwned + Clone + Debug;

    fn clear(&mut self) -> io::Result<()>;

    fn get_all<T>(&mut self) -> io::Result<Vec<T>>
    where
        T: Serialize + DeserializeOwned + Clone + Debug;

    fn get_many<T>(&mut self, keys: Vec<String>) -> io::Result<Vec<T>>
    where
        T: Serialize + DeserializeOwned + Clone + Debug;

    fn set_many<T>(&mut self, values: Vec<BinaryKv<T>>) -> io::Result<()>
    where
        T: Serialize + DeserializeOwned + Clone + Debug;

    fn delete_many<T>(&mut self, keys: Vec<String>) -> io::Result<()>
    where
        T: Serialize + DeserializeOwned + Clone + Debug;

    fn update_many<T>(&mut self, values: Vec<BinaryKv<T>>) -> io::Result<()>
    where
        T: Serialize + DeserializeOwned + Clone + Debug;
}
