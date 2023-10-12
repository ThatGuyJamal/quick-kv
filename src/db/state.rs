use std::collections::BTreeSet;
use std::fmt::Debug;
use std::hash::Hash;

use chrono::{DateTime, Utc};
use serde::de::DeserializeOwned;
use serde::Serialize;

use crate::db::entry::Entry;
use crate::types::HashMap;

#[derive(Debug, Clone)]
pub(crate) struct State<T>
where
    T: Serialize + DeserializeOwned + Debug + Eq + PartialEq + Hash + Send + Sync + Clone,
{
    /// The key-value store entries in memory
    pub(crate) entries: HashMap<String, Entry<T>>,

    /// Tracks key TTLs.
    ///
    /// A `BTreeSet` is used to maintain expirations sorted by when they expire.
    /// This allows the background task to iterate this map to find the value
    /// expiring next.
    ///
    /// While highly unlikely, it is possible for more than one expiration to be
    /// created for the same instant. Because of this, the `DateTime` is
    /// insufficient for the key. A unique key (`String`) is used to
    /// break these ties.
    pub(crate) expirations: BTreeSet<(DateTime<Utc>, String)>,
}

impl<T> State<T>
where
    T: Serialize + DeserializeOwned + Debug + Eq + PartialEq + Hash + Send + Sync + Clone,
{
    pub(crate) fn new() -> Self
    {
        Self {
            entries: HashMap::default(),
            expirations: BTreeSet::new(),
        }
    }
}
