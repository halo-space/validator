mod unique;

use std::borrow::Cow;
use std::collections::HashSet;
use std::hash::Hash;
use std::time::SystemTime;

use crate::core::Items;
use crate::{Error, Kind, Value};

pub(super) use unique::Unique;

#[derive(Eq, Hash, PartialEq)]
enum UniqueKey<'a> {
    None,
    String(Cow<'a, str>),
    Bool(bool),
    Int(i128),
    Uint(u128),
    Float(u64),
    Time(SystemTime),
}

pub(crate) fn values_are_unique<'a>(
    items: impl IntoIterator<Item = &'a dyn Value>,
) -> Result<bool, Error> {
    let mut seen = HashSet::new();

    for item in items {
        match insert(&mut seen, item) {
            Ok(true) => {}
            Ok(false) => return Ok(false),
            Err(kind) => {
                return Err(Error::InvalidRuleExpression {
                    expression: "unique".to_owned(),
                    reason: format!(
                        "collection item with kind {kind:?} cannot be used as a unique key"
                    ),
                });
            }
        }
    }

    Ok(true)
}

pub(crate) fn fields_are_unique(items: &dyn Items, field: &str) -> Result<bool, Error> {
    let mut seen = HashSet::new();
    let mut unique = true;
    let mut invalid = None;

    items.visit(field, &mut |value| {
        let key = match value {
            None => Some(UniqueKey::None),
            Some(value) => match insert(&mut seen, value) {
                Ok(true) => return true,
                Ok(false) => {
                    unique = false;
                    return false;
                }
                Err(kind) => {
                    invalid = Some(kind);
                    return false;
                }
            },
        };
        let Some(key) = key else {
            invalid = value.map(Value::kind);
            return false;
        };
        if !seen.insert(key) {
            unique = false;
            return false;
        }
        true
    })?;

    if let Some(kind) = invalid {
        return Err(Error::InvalidRuleExpression {
            expression: "unique".to_owned(),
            reason: format!("field '{field}' with kind {kind:?} cannot be used as a unique key"),
        });
    }

    Ok(unique)
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
        Kind::Time => value.time().map(UniqueKey::Time),
        Kind::Vec | Kind::Array | Kind::Slice | Kind::Map | Kind::Option | Kind::Other => None,
    }
}

fn insert<'a>(seen: &mut HashSet<UniqueKey<'a>>, value: &'a dyn Value) -> Result<bool, Kind> {
    if matches!(value.kind(), Kind::Float(_)) && value.float().is_some_and(f64::is_nan) {
        return Ok(true);
    }

    unique_key(value)
        .map(|key| seen.insert(key))
        .ok_or_else(|| value.kind())
}

fn float_key(value: f64) -> u64 {
    let value = if value == 0.0 { 0.0 } else { value };

    value.to_bits()
}
