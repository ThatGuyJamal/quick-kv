use std::path::Path;
use std::time::Duration;

use anyhow::Ok;
use log::LevelFilter;

use super::runtime::{RunTime, RuntTimeType};

/// The configuration for the database.
#[derive(Debug)]
pub struct DatabaseConfiguration
{
    /// The path to the database file.
    ///
    /// Default: "db.qkv"
    pub(crate) path: Option<String>,
    /// The type of run-time to use for the database.
    ///
    /// Default: RuntTimeType::Disk
    pub(crate) runtime: Option<RunTime>,
    /// If the database should log to stdout.
    ///
    /// Default: true
    pub(crate) log: Option<bool>,
    /// The log level to use for the database.
    ///
    /// Default: LevelFilter::Info
    pub(crate) log_level: Option<LevelFilter>,
    /// The default time-to-live for entries in the database.
    ///
    /// If enabled, all entries will have a ttl by default.
    /// If disabled (None), then you will have to manually set the ttl for each entry.
    ///
    /// Default: None
    pub(crate) default_ttl: Option<Duration>,
}

impl DatabaseConfiguration
{
    pub fn new(
        path: Option<String>,
        runtime: Option<RunTime>,
        log: Option<bool>,
        log_level: Option<LevelFilter>,
        default_ttl: Option<Duration>,
    ) -> anyhow::Result<Self>
    {
        let vp = match path {
            Some(p) => validate_path(p.as_str()),
            None => "db.qkv".to_string(),
        };

        // Extract the directory part from the path
        let dir_path = Path::new(&vp).parent().unwrap_or_else(|| Path::new(""));

        // Create the parent directories if they don't exist
        if !dir_path.exists() {
            std::fs::create_dir_all(dir_path)?;
        }

        Ok(Self {
            path: Some(vp.to_string()),
            runtime,
            log,
            log_level,
            default_ttl,
        })
    }
}

/// Used to validate if the database path is valid.
/// If not it will apply the appropriate changes to make it valid.
fn validate_path(input: &str) -> String
{
    let mut result = String::from(input);

    if input.ends_with('/') {
        // It's a directory path, so append "db.qkv" to it
        result.push_str("db.qkv");
    } else if !input.contains('.') {
        // It doesn't have an extension, so add ".qkv"
        result.push_str(".qkv");
    } else if !input.ends_with(".qkv") {
        // Ensure it ends with ".qkv"
        let index = input.rfind('.').unwrap_or(0);
        result.replace_range(index.., ".qkv");
    }

    result
}

impl Default for DatabaseConfiguration
{
    fn default() -> Self
    {
        Self {
            path: Some("db.qkv".to_string()),
            runtime: Some(RunTime::new(RuntTimeType::Disk)),
            log: Some(true),
            log_level: None,
            default_ttl: None,
        }
    }
}
