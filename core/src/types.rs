use rustc_hash::{FxHashMap, FxHashSet};

// Type aliases for the Hashing. This is to make it easier to change the hashing algorithm in the future
// as we don't need cryptographic security in our offline db.
pub type HashMap<K, V> = std::collections::HashMap<K, V, std::hash::BuildHasherDefault<FxHashMap<K, V>>>;
pub type HashSet<V> = std::collections::HashSet<std::hash::BuildHasherDefault<FxHashSet<V>>>;

// The different types of run-times that can be used for the database.
// Disk will both cache and write to disk, while memory will only cache.
#[derive(Debug, Clone)]
pub enum RuntTimeType {
    Memory,
    Disk,
}
