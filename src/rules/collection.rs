mod unique;

use std::collections::HashSet;

use crate::{Kind, Value};

pub(super) use unique::Unique;

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

fn unique_key(value: &dyn Value) -> Option<String> {
    if value.is_none() {
        return Some("none".to_owned());
    }

    match value.kind() {
        Kind::String => value.string().map(|value| format!("s:{}", value.as_ref())),
        Kind::Bool => value.boolean().map(|value| format!("b:{value}")),
        Kind::Int(_) => value.int().map(|value| format!("i:{value}")),
        Kind::Uint(_) => value.uint().map(|value| format!("u:{value}")),
        Kind::Float(_) => value.float().map(float_key),
        Kind::Vec
        | Kind::Array
        | Kind::Slice
        | Kind::Map
        | Kind::Option
        | Kind::Time
        | Kind::Other => None,
    }
}

fn float_key(value: f64) -> String {
    let value = if value == 0.0 { 0.0 } else { value };

    format!("f:{}", value.to_bits())
}
