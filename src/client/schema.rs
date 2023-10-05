use crate::types::BinaryKv;
use serde::de::DeserializeOwned;
use serde::Serialize;
use std::collections::HashMap;
use std::fmt::Debug;
use std::fs::File;
use std::sync::{Arc, Mutex};

/// The Schema client is a more optimized and faster version of the normal client.
///
/// It allows you to define a schema for your data, which will be used to serialize and deserialize your data.
/// The benefit is all operations are optimized for your data type, it also makes typings easier to work with.
/// Use this client when you want to work with data-modules that you have defined. The normal client is good
/// for storing generic data that could change frequently.
#[derive(Debug)]
pub struct QuickSchemaClient<T>
where
    T: Serialize + DeserializeOwned + Clone + Debug,
{
    pub file: Arc<Mutex<File>>,
    pub cache: Mutex<HashMap<String, BinaryKv<T>>>,
}
