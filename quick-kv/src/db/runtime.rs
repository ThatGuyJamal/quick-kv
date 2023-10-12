// The different types of run-times that can be used for the database.
// Disk will both cache and write to disk, while memory will only cache.
#[derive(Debug, Clone)]
pub enum RuntTimeType
{
    Memory,
    Disk,
}

/// Specifies the type of run-time to use for the database.
///
/// This controls how the database modules will be stored,
/// and optimized for.
#[derive(Debug, Clone)]
pub struct RunTime
{
    /// The type of runtime to use for the database.
    ///
    /// Default: RuntTimeType::Disk
    pub _type: RuntTimeType,
}

impl RunTime
{
    pub fn new(_type: RuntTimeType) -> Self
    {
        Self { _type }
    }

    /// Get the type of run-time.
    pub(crate) fn get_type(&self) -> &RuntTimeType
    {
        &self._type
    }
}
