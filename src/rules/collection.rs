mod unique;

use std::borrow::Cow;
use std::collections::HashSet;
use std::hash::Hash;

use crate::{Kind, Value};

pub(super) use unique::Unique;

#[derive(Eq, Hash, PartialEq)]
enum UniqueKey<'a> {
    None,
    String(Cow<'a, str>),
    Bool(bool),
    Int(i128),
    Uint(u128),
    Float(u64),
}

pub(crate) fn values_are_unique<'a>(items: impl IntoIterator<Item = &'a dyn Value>) -> bool {
    let mut seen = HashSet::new();

    for item in items {
        let Some(key) = unique_key(item) else {
            return false;
        };

        if !seen.insert(key) {
            return false;
        }
    }

    true
}

fn unique_key(value: &dyn Value) -> Option<UniqueKey<'_>> {
    if value.is_none() {
        return Some(UniqueKey::None);
    }

    match value.kind() {
        Kind::String => value.string().map(UniqueKey::String),
        Kind::Bool => value.boolean().map(UniqueKey::Bool),
        Kind::Int(_) => value.int().map(UniqueKey::Int),
        Kind::Uint(_) => value.uint().map(UniqueKey::Uint),
        Kind::Float(_) => value.float().map(float_key).map(UniqueKey::Float),
        Kind::Vec
        | Kind::Array
        | Kind::Slice
        | Kind::Map
        | Kind::Option
        | Kind::Time
        | Kind::Other => None,
    }
}

fn float_key(value: f64) -> u64 {
    let value = if value == 0.0 { 0.0 } else { value };

    value.to_bits()
}
