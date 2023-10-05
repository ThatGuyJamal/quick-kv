use serde::{Deserialize, Deserializer, Serialize};
use std::collections::HashMap;
use std::fmt::Debug;

/// The data structure used to store key-value pairs in the database
#[derive(Serialize, PartialEq, Debug, Clone)]
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
    pub(crate) fn new(key: String, value: T) -> Self {
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
        struct ValueHelper<T> {
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

/// Represents a value that can be stored in the database.
///
/// This can be any type of data that implements `Serialize` and `Deserialize` from the `serde` crate.
#[derive(Serialize, Deserialize, PartialEq, Debug, Clone)]
pub enum Value {
    String(String),
    Bool(bool),
    U8(u8),
    U16(u16),
    U32(u32),
    U64(u64),
    U128(u128),
    Usize(usize),
    I8(i8),
    I16(i16),
    I32(i32),
    I64(i64),
    I128(i128),
    Isize(isize),
    F32(f32),
    F64(f64),
    None,
}

pub trait IntoValue {
    fn into_value(self) -> Value;
    fn into_string(self) -> String;
    fn into_bool(self) -> bool;
    fn into_u8(self) -> u8;
    fn into_u16(self) -> u16;
    fn into_u32(self) -> u32;
    fn into_u64(self) -> u64;
    fn into_u128(self) -> u128;
    fn into_usize(self) -> usize;
    fn into_i8(self) -> i8;
    fn into_i16(self) -> i16;
    fn into_i32(self) -> i32;
    fn into_i64(self) -> i64;
    fn into_i128(self) -> i128;
    fn into_isize(self) -> isize;
    fn into_f32(self) -> f32;
    fn into_f64(self) -> f64;
}

impl IntoValue for Value {
    fn into_value(self) -> Value {
        self
    }

    fn into_string(self) -> String {
        match self {
            Value::String(string) => string,
            _ => panic!("Cannot convert Value to String"),
        }
    }

    fn into_bool(self) -> bool {
        match self {
            Value::Bool(bool) => bool,
            _ => panic!("Cannot convert Value to bool"),
        }
    }

    fn into_u8(self) -> u8 {
        match self {
            Value::U8(u8) => u8,
            _ => panic!("Cannot convert Value to u8"),
        }
    }

    fn into_u16(self) -> u16 {
        match self {
            Value::U16(u16) => u16,
            _ => panic!("Cannot convert Value to u16"),
        }
    }

    fn into_u32(self) -> u32 {
        match self {
            Value::U32(u32) => u32,
            _ => panic!("Cannot convert Value to u32"),
        }
    }

    fn into_u64(self) -> u64 {
        match self {
            Value::U64(u64) => u64,
            _ => panic!("Cannot convert Value to u64"),
        }
    }

    fn into_u128(self) -> u128 {
        match self {
            Value::U128(u128) => u128,
            _ => panic!("Cannot convert Value to u128"),
        }
    }

    fn into_usize(self) -> usize {
        match self {
            Value::Usize(usize) => usize,
            _ => panic!("Cannot convert Value to usize"),
        }
    }

    fn into_i8(self) -> i8 {
        match self {
            Value::I8(i8) => i8,
            _ => panic!("Cannot convert Value to i8"),
        }
    }

    fn into_i16(self) -> i16 {
        match self {
            Value::I16(i16) => i16,
            _ => panic!("Cannot convert Value to i16"),
        }
    }

    fn into_i32(self) -> i32 {
        match self {
            Value::I32(i32) => i32,
            _ => panic!("Cannot convert Value to i32"),
        }
    }

    fn into_i64(self) -> i64 {
        match self {
            Value::I64(i64) => i64,
            _ => panic!("Cannot convert Value to i64"),
        }
    }

    fn into_i128(self) -> i128 {
        match self {
            Value::I128(i128) => i128,
            _ => panic!("Cannot convert Value to i128"),
        }
    }

    fn into_isize(self) -> isize {
        match self {
            Value::Isize(isize) => isize,
            _ => panic!("Cannot convert Value to isize"),
        }
    }

    fn into_f32(self) -> f32 {
        match self {
            Value::F32(f32) => f32,
            _ => panic!("Cannot convert Value to f32"),
        }
    }

    fn into_f64(self) -> f64 {
        match self {
            Value::F64(f64) => f64,
            _ => panic!("Cannot convert Value to f64"),
        }
    }
}

/// A util trait for converting a raw type into a Value
pub trait RawIntoValue {
    fn into_value(self) -> Value;
}

impl RawIntoValue for String {
    fn into_value(self) -> Value {
        Value::String(self)
    }
}

impl RawIntoValue for bool {
    fn into_value(self) -> Value {
        Value::Bool(self)
    }
}

impl RawIntoValue for u8 {
    fn into_value(self) -> Value {
        Value::U8(self)
    }
}

impl RawIntoValue for u16 {
    fn into_value(self) -> Value {
        Value::U16(self)
    }
}

impl RawIntoValue for u32 {
    fn into_value(self) -> Value {
        Value::U32(self)
    }
}

impl RawIntoValue for u64 {
    fn into_value(self) -> Value {
        Value::U64(self)
    }
}

impl RawIntoValue for u128 {
    fn into_value(self) -> Value {
        Value::U128(self)
    }
}

impl RawIntoValue for usize {
    fn into_value(self) -> Value {
        Value::Usize(self)
    }
}

impl RawIntoValue for i8 {
    fn into_value(self) -> Value {
        Value::I8(self)
    }
}

impl RawIntoValue for i16 {
    fn into_value(self) -> Value {
        Value::I16(self)
    }
}

impl RawIntoValue for i32 {
    fn into_value(self) -> Value {
        Value::I32(self)
    }
}

impl RawIntoValue for i64 {
    fn into_value(self) -> Value {
        Value::I64(self)
    }
}

impl RawIntoValue for i128 {
    fn into_value(self) -> Value {
        Value::I128(self)
    }
}

impl RawIntoValue for isize {
    fn into_value(self) -> Value {
        Value::Isize(self)
    }
}

impl RawIntoValue for f32 {
    fn into_value(self) -> Value {
        Value::F32(self)
    }
}

impl RawIntoValue for f64 {
    fn into_value(self) -> Value {
        Value::F64(self)
    }
}

/// Represents a typed value that can be stored in the database.
///
/// This type is used for Vectors and HashMaps for generic support.
#[derive(Serialize, Deserialize, PartialEq, Debug, Clone)]
pub enum TypedValue<T> {
    Vec(Vec<T>),
    Hash(HashMap<String, T>),
}

pub trait IntoTypedValue<T> {
    fn into_vec(self) -> Vec<T>;
    fn into_hash(self) -> HashMap<String, T>;
}

impl<T> IntoTypedValue<T> for TypedValue<T> {
    fn into_vec(self) -> Vec<T> {
        match self {
            TypedValue::Vec(vec) => vec,
            _ => panic!("Cannot convert TypedValue to Vec"),
        }
    }

    fn into_hash(self) -> HashMap<String, T> {
        match self {
            TypedValue::Hash(hash) => hash,
            _ => panic!("Cannot convert TypedValue to HashMap"),
        }
    }
}

/// A util trait for converting a raw type into a TypedValue
pub trait RawIntoTypedValue<T> {
    fn into_typed(self) -> TypedValue<T>;
}

impl<T> RawIntoTypedValue<T> for Vec<T> {
    fn into_typed(self) -> TypedValue<T> {
        TypedValue::Vec(self)
    }
}

impl<T> RawIntoTypedValue<T> for HashMap<String, T> {
    fn into_typed(self) -> TypedValue<T> {
        TypedValue::Hash(self)
    }
}
