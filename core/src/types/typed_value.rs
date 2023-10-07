use std::collections::HashMap;
use std::fmt::Debug;

use serde::{Deserialize, Serialize};

/// Represents any type of data that can be stored in the database.
///
/// The only different between this and `Value` is that this is a generic type, and `Value` is not.
/// This type also only supports `Vec<T>`, `HashMap<String, T>`, and `Option<T>`.
/// ```rust
/// use quick_kv::prelude::*;
///
/// let mut list_of_people = vec!["Ray".to_string(), "Noa".to_string(), "Kian".to_string()];
///
/// let typed_value = TypedValue::<String>::Vec(list_of_people.clone());
/// ```
///
/// You can also convert a `TypedValue` into a `Vec<T>`, `HashMap<String, T>`, or `Option<T>`.
/// ```rust
/// use quick_kv::prelude::*;
///
/// let mut list_of_people = vec!["Ray".to_string(), "Noa".to_string(), "Kian".to_string()];
///
/// let typed_value_as_a_vec = TypedValue::<String>::Vec(list_of_people.clone()).into_vec();
/// ```
/// These are not really practical examples, but the TypedValue enum is useful when working with
/// the normal client and when needing to ensure type safety on data operations.
#[derive(Serialize, Deserialize, PartialEq, Debug, Clone)]
pub enum TypedValue<T>
{
    Vec(Vec<T>),
    Hash(HashMap<String, T>),
    Option(Option<T>),
}

/// A util trait for converting a TypedValue a usable type in rust.
pub trait IntoTypedValue<T>
{
    fn into_vec(self) -> Vec<T>;
    fn into_hash(self) -> HashMap<String, T>;
    fn into_option(self) -> Option<T>;
}

impl<T> IntoTypedValue<T> for TypedValue<T>
{
    fn into_vec(self) -> Vec<T>
    {
        match self {
            TypedValue::Vec(vec) => vec,
            _ => panic!("Cannot convert TypedValue to Vec"),
        }
    }

    fn into_hash(self) -> HashMap<String, T>
    {
        match self {
            TypedValue::Hash(hash) => hash,
            _ => panic!("Cannot convert TypedValue to HashMap"),
        }
    }

    fn into_option(self) -> Option<T>
    {
        match self {
            TypedValue::Option(option) => option,
            _ => panic!("Cannot convert TypedValue to Option"),
        }
    }
}

/// A util trait for converting a raw type into a TypedValue
///
/// ```rust
/// use std::collections::HashMap;
///
/// use quick_kv::prelude::*;
///
/// let typed_hashmap = HashMap::<String, i32>::new().into_typed();
/// ```
pub trait RawIntoTypedValue<T>
{
    fn into_typed(self) -> TypedValue<T>;
}

impl<T> RawIntoTypedValue<T> for Vec<T>
{
    fn into_typed(self) -> TypedValue<T>
    {
        TypedValue::Vec(self)
    }
}

impl<T> RawIntoTypedValue<T> for HashMap<String, T>
{
    fn into_typed(self) -> TypedValue<T>
    {
        TypedValue::Hash(self)
    }
}

impl<T> RawIntoTypedValue<T> for Option<T>
{
    fn into_typed(self) -> TypedValue<T>
    {
        TypedValue::Option(self)
    }
}
