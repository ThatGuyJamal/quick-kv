use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, PartialEq, Debug, Clone)]
pub enum Value {
    String(String),
    I64(i64),
    I32(i32),
    U64(u64),
    U32(u32),
    F64(f64),
    F32(f32),
    Boolean(bool),
    Object(Vec<(String, Value)>),
    Array(Vec<Value>),
    Null,
}

#[derive(Debug, Clone)]
pub struct QuickKVConfig {
    pub db_file: Option<String>,
    pub max_db_size: Option<u64>,
}

impl Default for QuickKVConfig {
    fn default() -> Self {
        QuickKVConfig {
            db_file: "db.qkv".to_string().into(),
            max_db_size: None,
        }
    }
}