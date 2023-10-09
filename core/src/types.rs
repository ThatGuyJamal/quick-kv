use rustc_hash::{FxHashMap, FxHashSet};

// Type aliases for the Hashing. This is to make it easier to change the hashing algorithm in the future
// as we don't need cryptographic security in our offline db.

// For HashMap
pub type HashMap<K, V> = FxHashMap<K, V>;

// For HashSet
pub type HashSet<V> = FxHashSet<V>;
