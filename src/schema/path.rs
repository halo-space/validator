use serde_json::{Map, Value};
use unicode_ident::{is_xid_continue, is_xid_start};

use crate::Error;

pub(super) fn namespace(parent: &str, field: &str) -> String {
    if parent.is_empty() {
        field.to_owned()
    } else {
        format!("{parent}.{field}")
    }
}

pub(super) fn resolve<'a>(
    object: &'a Map<String, Value>,
    segments: &[String],
) -> Option<&'a Value> {
    let mut value = object.get(segments.first()?)?;
    for segment in &segments[1..] {
        value = value.as_object()?.get(segment)?;
    }
    Some(value)
}

pub(super) fn parse_path(attribute_name: &str, path: &str) -> Result<Vec<String>, Error> {
    if path.is_empty() {
        return Err(invalid_path(attribute_name, path));
    }

    path.split('.')
        .map(|segment| {
            if is_identifier(segment) {
                Ok(segment.to_owned())
            } else {
                Err(invalid_path(attribute_name, path))
            }
        })
        .collect()
}

fn is_identifier(segment: &str) -> bool {
    if segment.is_empty()
        || segment.starts_with("r#")
        || matches!(segment, "_" | "Self" | "crate" | "self" | "super")
    {
        return false;
    }

    let mut chars = segment.chars();
    let Some(first) = chars.next() else {
        return false;
    };
    (first == '_' || is_xid_start(first)) && chars.all(is_xid_continue)
}

fn invalid_path(attribute_name: &str, path: &str) -> Error {
    Error::InvalidSchema {
        reason: format!(
            "field rule '{attribute_name}' has invalid field path '{path}'; expected dot-separated Rust field identifiers"
        ),
    }
}
