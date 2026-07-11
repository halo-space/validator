use std::collections::BTreeMap;

use serde_json::Value as JsonValue;

use super::FieldDef;
use crate::Error;
use crate::core::{Expr, Spec, parse_expression};

pub(super) fn fields(parent: &str, value: &JsonValue) -> Result<BTreeMap<String, FieldDef>, Error> {
    let object = value
        .as_object()
        .ok_or_else(|| invalid(format!("field '{parent}' nested fields must be an object")))?;

    object
        .iter()
        .map(|(name, field)| Ok((name.clone(), FieldDef::from_value(name, field)?)))
        .collect()
}

pub(super) fn expressions(field: &str, value: &JsonValue) -> Result<Vec<Expr>, Error> {
    match value {
        JsonValue::Array(expressions) => expressions
            .iter()
            .map(|value| expression_item(field, value))
            .collect::<Result<Vec<_>, _>>()
            .map(|exprs| exprs.into_iter().flatten().collect()),
        JsonValue::String(expr) => parse_expression(expr),
        _ => Err(invalid(format!(
            "field '{field}' rules must be a string or array"
        ))),
    }
}

fn expression_item(field: &str, value: &JsonValue) -> Result<Vec<Expr>, Error> {
    match value {
        JsonValue::String(expr) => parse_expression(expr),
        JsonValue::Object(object) => {
            if object.len() != 1 {
                return Err(invalid(format!(
                    "field '{field}' rule object must contain exactly one rule"
                )));
            }

            let (name, params) = object
                .iter()
                .next()
                .expect("rule object must contain one entry");
            Ok(vec![Expr::Single(spec(name, params)?)])
        }
        _ => Err(invalid(format!(
            "field '{field}' rule item must be a string or object"
        ))),
    }
}

fn spec(name: &str, params: &JsonValue) -> Result<Spec, Error> {
    if params.is_null() {
        return Ok(Spec::new(name));
    }

    if let Some(object) = params.as_object() {
        let mut spec = Spec::new(name);
        for (key, value) in object {
            spec = match value {
                JsonValue::Array(values) => spec.named_list(key, param_list(values)?),
                _ => spec.named(key, param(value)?),
            };
        }
        return Ok(spec);
    }

    match params {
        JsonValue::Array(values) if values.is_empty() => Err(invalid(format!(
            "rule '{name}' parameter list cannot be empty"
        ))),
        JsonValue::Array(values) => {
            let mut spec = Spec::new(name);
            for value in param_list(values)? {
                spec = spec.positional(value);
            }
            Ok(spec)
        }
        _ => Ok(Spec::new(name).positional(param(params)?)),
    }
}

fn param(value: &JsonValue) -> Result<String, Error> {
    match value {
        JsonValue::String(value) => Ok(value.clone()),
        JsonValue::Number(value) => Ok(value.to_string()),
        JsonValue::Bool(value) => Ok(value.to_string()),
        JsonValue::Null | JsonValue::Array(_) | JsonValue::Object(_) => Err(invalid(
            "rule parameter must be a string, number, or boolean",
        )),
    }
}

fn param_list(values: &[JsonValue]) -> Result<Vec<String>, Error> {
    values.iter().map(param).collect()
}

pub(super) fn reject_unknown_keys(
    object: &serde_json::Map<String, JsonValue>,
    allowed: &[&str],
    context: &str,
) -> Result<(), Error> {
    if let Some(key) = object.keys().find(|key| !allowed.contains(&key.as_str())) {
        return Err(invalid(format!("{context} uses unknown key '{key}'")));
    }

    Ok(())
}

pub(super) fn invalid(reason: impl Into<String>) -> Error {
    Error::InvalidSchema {
        reason: reason.into(),
    }
}
