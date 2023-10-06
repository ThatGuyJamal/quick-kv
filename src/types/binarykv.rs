use std::fmt::Debug;

use serde::{Deserialize, Deserializer, Serialize};

/// BinaryKV is how data is represented programmatically in the database.
///
/// It accepts any type that implements `Serialize` and `Deserialize` from the `serde` crate
/// and uses it for the value of the key-value pair.
/// ```rust
/// use quick_kv::prelude::*;
///
/// BinaryKv::new("key".to_string(), "value".to_string());
/// ```
/// This is the same as:
/// ```rust
/// use quick_kv::prelude::*;
///
/// let data = BinaryKv {
///     key: "key".to_string(),
///     value: "value".to_string(),
/// };
/// ```
#[derive(Serialize, PartialEq, Debug, Clone, Eq, Hash, PartialOrd)]
pub struct BinaryKv<T>
where
    T: Serialize + Clone + Debug,
{
    /// The key of the key-value pair
    pub key: String,
    /// The value of the key-value pair
    pub value: T,
}

impl<T> BinaryKv<T>
where
    T: Serialize + Clone + Debug,
{
    pub fn new(key: String, value: T) -> Self
    {
        BinaryKv { key, value }
    }
}

impl<'de, T> Deserialize<'de> for BinaryKv<T>
where
    T: Deserialize<'de> + Serialize + Clone + Debug,
{
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        #[derive(Deserialize)]
        struct ValueHelper<T>
        {
            key: String,
            value: T,
        }

        let helper = ValueHelper::<T>::deserialize(deserializer)?;

        Ok(BinaryKv {
            key: helper.key,
            value: helper.value,
        })
    }
}
