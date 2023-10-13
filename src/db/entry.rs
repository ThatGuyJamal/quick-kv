use std::fmt::Debug;
use std::hash::Hash;

use chrono::{DateTime, Utc};
use serde::de::DeserializeOwned;
use serde::{Deserialize, Deserializer, Serialize};

/// Entry in the key-value store
#[derive(Debug, Serialize, Clone)]
pub(crate) struct Entry<T>
where
    T: Serialize + DeserializeOwned + Debug + Eq + PartialEq + Hash + Send + Sync,
{
    pub(crate) key: String,
    /// Stored data
    pub(crate) data: T,
    /// Instant at which the entry expires and should be removed from the
    /// database.
    pub(crate) expires_at: Option<DateTime<Utc>>,
}

impl<T> Entry<T>
where
    T: Serialize + DeserializeOwned + Debug + Eq + PartialEq + Hash + Send + Sync,
{
    pub(crate) fn new(key: String, data: T, expires_at: Option<DateTime<Utc>>) -> Self
    {
        Self { key, data, expires_at }
    }
}

impl<'de, T> Deserialize<'de> for Entry<T>
where
    T: Serialize + DeserializeOwned + Debug + Eq + PartialEq + Hash + Send + Sync,
{
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        #[derive(Deserialize)]
        struct EntryHelper<T>
        {
            key: String,
            data: T,
            expires_at: Option<DateTime<Utc>>,
        }

        let helper = EntryHelper::<T>::deserialize(deserializer)?;

        Ok(Self {
            key: helper.key,
            data: helper.data,
            expires_at: helper.expires_at,
        })
    }
}
