use std::{
    collections::BTreeSet,
    sync::{Arc, Mutex},
};

use bytes::Bytes;
use log::LevelFilter;
use time::Instant;

use crate::types::{HashMap, RuntTimeType};

#[derive(Debug, Clone)]
pub struct DatabaseConfiguration<'a> {
    /// The path to the database file.
    path: Option<&'a str>,
    /// The type of run-time to use for the database.
    runtime: Option<RuntTimeType>,
    log: Option<bool>,
    log_level: Option<LevelFilter>,
}

impl<'a> DatabaseConfiguration<'a> {
    pub fn new(
        path: Option<&'a str>,
        runtime: Option<RuntTimeType>,
        log: Option<bool>,
        log_level: Option<LevelFilter>,
    ) -> Self {
        Self {
            path,
            runtime,
            log,
            log_level,
        }
    }
}

impl Default for DatabaseConfiguration<'_> {
    fn default() -> Self {
        Self {
            path: Some("db.qkv"),
            runtime: Some(RuntTimeType::Disk),
            log: Some(false),
            log_level: None,
        }
    }
}

#[derive(Debug, Clone)]
pub(crate) struct Database {
    state: Arc<Mutex<State>>,
}

impl Database {
    pub(crate) fn new() -> Self {
        Self {
            state: Arc::new(Mutex::new(State {
                entries: HashMap::default(),
                expirations: BTreeSet::default(),
            })),
        }
    }
}

#[derive(Debug)]
struct State {
    /// The key-value store entries in memory
    entries: HashMap<String, Entry>,

    /// Tracks key TTLs.
    ///
    /// A `BTreeSet` is used to maintain expirations sorted by when they expire.
    /// This allows the background task to iterate this map to find the value
    /// expiring next.
    ///
    /// While highly unlikely, it is possible for more than one expiration to be
    /// created for the same instant. Because of this, the `Instant` is
    /// insufficient for the key. A unique key (`String`) is used to
    /// break these ties.
    expirations: BTreeSet<(Instant, String)>,
}

/// Entry in the key-value store
#[derive(Debug)]
struct Entry {
    /// Stored data
    data: Bytes,
    /// Instant at which the entry expires and should be removed from the
    /// database.
    expires_at: Option<Instant>,
}
