mod eq;
mod eq_ignore_case;
mod gt;
mod gte;
mod lt;
mod lte;
mod ne;
mod ne_ignore_case;

use std::cmp::Ordering;

use crate::{Error, Field, FloatKind, Kind};

pub(super) use eq::Eq;
pub(super) use eq_ignore_case::EqIgnoreCase;
pub(super) use gt::Gt;
pub(super) use gte::Gte;
pub(super) use lt::Lt;
pub(super) use lte::Lte;
pub(super) use ne::Ne;
pub(super) use ne_ignore_case::NeIgnoreCase;

#[derive(Clone, Copy, Debug)]
pub(super) enum Relation {
    Eq,
    Gt,
    Gte,
    Lt,
    Lte,
}

pub(super) fn satisfies(
    field: &Field<'_>,
    limit_name: &str,
    relation: Relation,
) -> Result<bool, Error> {
    validate_satisfies(field, limit_name)?;
    let limit = field.params().text(limit_name);

    if field.value().kind() == Kind::Time {
        if let Some(limit) = limit {
            return Err(invalid_time_parameter(limit_name, limit));
        }

        return Ok(limit_name == "value" && time_satisfies_now(field, relation));
    }

    let limit = limit.ok_or_else(|| Error::InvalidRuleExpression {
        expression: limit_name.to_owned(),
        reason: format!("rule requires exactly one '{limit_name}' parameter for this field type"),
    })?;

    value_satisfies(field, limit_name, limit, relation)
}

pub(super) fn equals(field: &Field<'_>) -> Result<bool, Error> {
    equality(field, "eq")
}

pub(super) fn not_equals(field: &Field<'_>) -> Result<bool, Error> {
    equality(field, "ne").map(|equal| !equal)
}

fn equality(field: &Field<'_>, rule: &str) -> Result<bool, Error> {
    validate_equality(field, rule)?;

    if field.value().kind() == Kind::Time {
        unreachable!("time equality is rejected during parameter validation");
    }

    value_equals(
        field,
        field
            .params()
            .text("value")
            .expect("value parameter is checked above"),
    )
}

pub(super) fn validate_equality(field: &Field<'_>, rule: &str) -> Result<(), Error> {
    let value = field.params().text("value");
    match field.value().kind() {
        Kind::Time => {
            if let Some(value) = value {
                Err(invalid_time_parameter("value", value))
            } else {
                Err(invalid_time_equality(rule))
            }
        }
        Kind::String | Kind::Option => value.map_or_else(
            || {
                Err(Error::InvalidRuleExpression {
                    expression: rule.to_owned(),
                    reason: "rule requires exactly one 'value' parameter".to_owned(),
                })
            },
            |_| Ok(()),
        ),
        Kind::Bool => parse_limit("value", value, |value| value.parse::<bool>(), "boolean"),
        Kind::Vec | Kind::Array | Kind::Slice | Kind::Map | Kind::Int(_) => {
            parse_limit("value", value, str::parse::<i128>, "int")
        }
        Kind::Uint(_) => parse_limit("value", value, str::parse::<u128>, "uint"),
        Kind::Float(FloatKind::F32) => parse_limit("value", value, str::parse::<f32>, "f32"),
        Kind::Float(FloatKind::F64) => parse_limit("value", value, str::parse::<f64>, "f64"),
        Kind::Other => Err(invalid_kind("value", field.value().kind())),
    }
}

pub(super) fn validate_satisfies(field: &Field<'_>, name: &str) -> Result<(), Error> {
    let value = field.params().text(name);
    match field.value().kind() {
        Kind::Time => match value {
            Some(value) => Err(invalid_time_parameter(name, value)),
            None if name == "value" => Ok(()),
            None => Err(missing_limit(name)),
        },
        Kind::String | Kind::Vec | Kind::Array | Kind::Slice | Kind::Map | Kind::Int(_) => {
            parse_limit(name, value, str::parse::<i128>, "int")
        }
        Kind::Uint(_) => parse_limit(name, value, str::parse::<u128>, "uint"),
        Kind::Float(FloatKind::F32) => parse_limit(name, value, str::parse::<f32>, "f32"),
        Kind::Float(FloatKind::F64) => parse_limit(name, value, str::parse::<f64>, "f64"),
        Kind::Option => {
            let Some(value) = value else {
                return Ok(());
            };
            if value.parse::<i128>().is_ok()
                || value.parse::<u128>().is_ok()
                || value.parse::<f64>().is_ok()
            {
                Ok(())
            } else {
                Err(invalid_limit(name, value, "number"))
            }
        }
        Kind::Bool | Kind::Other => Err(invalid_kind(name, field.value().kind())),
    }
}

pub(super) fn validate_bounds(field: &Field<'_>, rule: &str) -> Result<(), Error> {
    let min = field
        .params()
        .text("min")
        .ok_or_else(|| missing_limit("min"))?;
    let max = field
        .params()
        .text("max")
        .ok_or_else(|| missing_limit("max"))?;
    let ordered = match field.value().kind() {
        Kind::String | Kind::Vec | Kind::Array | Kind::Slice | Kind::Map | Kind::Int(_) => {
            signed_limit("min", min)? <= signed_limit("max", max)?
        }
        Kind::Uint(_) => unsigned_limit("min", min)? <= unsigned_limit("max", max)?,
        Kind::Float(FloatKind::F32) => f32_limit("min", min)? <= f32_limit("max", max)?,
        Kind::Float(FloatKind::F64) => f64_limit("min", min)? <= f64_limit("max", max)?,
        Kind::Option => unknown_bounds_are_ordered(min, max),
        Kind::Bool | Kind::Time | Kind::Other => {
            return Err(invalid_kind("min", field.value().kind()));
        }
    };

    if ordered {
        Ok(())
    } else {
        Err(Error::InvalidRuleExpression {
            expression: rule.to_owned(),
            reason: format!("minimum '{min}' must not be greater than maximum '{max}'"),
        })
    }
}

fn unknown_bounds_are_ordered(min: &str, max: &str) -> bool {
    if let (Ok(min), Ok(max)) = (min.parse::<i128>(), max.parse::<i128>()) {
        return min <= max;
    }
    if let (Ok(min), Ok(max)) = (min.parse::<u128>(), max.parse::<u128>()) {
        return min <= max;
    }

    matches!(
        (min.parse::<f64>(), max.parse::<f64>()),
        (Ok(min), Ok(max)) if min <= max
    )
}

fn parse_limit<T, E>(
    name: &str,
    value: Option<&str>,
    parse: impl FnOnce(&str) -> Result<T, E>,
    expected: &str,
) -> Result<(), Error> {
    let value = value.ok_or_else(|| missing_limit(name))?;
    parse(value)
        .map(|_| ())
        .map_err(|_| invalid_limit(name, value, expected))
}

fn missing_limit(name: &str) -> Error {
    Error::InvalidRuleExpression {
        expression: name.to_owned(),
        reason: format!("rule requires exactly one '{name}' parameter for this field type"),
    }
}

fn value_equals(field: &Field<'_>, limit: &str) -> Result<bool, Error> {
    match field.value().kind() {
        Kind::String => field
            .value()
            .string()
            .map(|value| value == limit)
            .ok_or_else(|| invalid_kind("value", field.value().kind())),
        Kind::Bool => {
            let limit = limit
                .parse::<bool>()
                .map_err(|_| invalid_limit("value", limit, "boolean"))?;
            field
                .value()
                .boolean()
                .map(|value| value == limit)
                .ok_or_else(|| invalid_kind("value", field.value().kind()))
        }
        Kind::Vec | Kind::Array | Kind::Slice | Kind::Map => {
            let limit = signed_limit("value", limit)?;
            field
                .value()
                .len()
                .map(|length| length as i128 == limit)
                .ok_or_else(|| invalid_kind("value", field.value().kind()))
        }
        Kind::Int(_) => {
            let limit = signed_limit("value", limit)?;
            field
                .value()
                .int()
                .map(|value| value == limit)
                .ok_or_else(|| invalid_kind("value", field.value().kind()))
        }
        Kind::Uint(_) => {
            let limit = unsigned_limit("value", limit)?;
            field
                .value()
                .uint()
                .map(|value| value == limit)
                .ok_or_else(|| invalid_kind("value", field.value().kind()))
        }
        Kind::Float(FloatKind::F32) => {
            let limit = f32_limit("value", limit)?;
            field
                .value()
                .float()
                .map(|value| value == limit)
                .ok_or_else(|| invalid_kind("value", field.value().kind()))
        }
        Kind::Float(FloatKind::F64) => {
            let limit = f64_limit("value", limit)?;
            field
                .value()
                .float()
                .map(|value| value == limit)
                .ok_or_else(|| invalid_kind("value", field.value().kind()))
        }
        Kind::Option | Kind::Time | Kind::Other => Err(invalid_kind("value", field.value().kind())),
    }
}

fn value_satisfies(
    field: &Field<'_>,
    name: &str,
    limit: &str,
    relation: Relation,
) -> Result<bool, Error> {
    match field.value().kind() {
        Kind::String => {
            let limit = signed_limit(name, limit)?;
            Ok(field
                .value()
                .len()
                .is_some_and(|length| signed_satisfies(length as i128, limit, relation)))
        }
        Kind::Vec | Kind::Array | Kind::Slice | Kind::Map => {
            let limit = signed_limit(name, limit)?;
            Ok(field
                .value()
                .len()
                .is_some_and(|length| signed_satisfies(length as i128, limit, relation)))
        }
        Kind::Int(_) => {
            let limit = signed_limit(name, limit)?;
            Ok(field
                .value()
                .int()
                .is_some_and(|value| signed_satisfies(value, limit, relation)))
        }
        Kind::Uint(_) => {
            let limit = unsigned_limit(name, limit)?;
            Ok(field
                .value()
                .uint()
                .is_some_and(|value| unsigned_satisfies(value, limit, relation)))
        }
        Kind::Float(FloatKind::F32) => {
            let limit = f32_limit(name, limit)?;
            Ok(field
                .value()
                .float()
                .is_some_and(|value| float_satisfies(value, limit, relation)))
        }
        Kind::Float(FloatKind::F64) => {
            let limit = f64_limit(name, limit)?;
            Ok(field
                .value()
                .float()
                .is_some_and(|value| float_satisfies(value, limit, relation)))
        }
        Kind::Bool | Kind::Option | Kind::Time | Kind::Other => {
            Err(invalid_kind(name, field.value().kind()))
        }
    }
}

fn time_satisfies_now(field: &Field<'_>, relation: Relation) -> bool {
    field
        .value()
        .time()
        .and_then(|value| value.partial_cmp(&field.now()))
        .is_some_and(|ordering| ordering_satisfies(ordering, relation))
}

fn signed_satisfies(value: i128, limit: i128, relation: Relation) -> bool {
    ordering_satisfies(value.cmp(&limit), relation)
}

fn unsigned_satisfies(value: u128, limit: u128, relation: Relation) -> bool {
    ordering_satisfies(value.cmp(&limit), relation)
}

fn float_satisfies(value: f64, limit: f64, relation: Relation) -> bool {
    value
        .partial_cmp(&limit)
        .is_some_and(|ordering| ordering_satisfies(ordering, relation))
}

fn ordering_satisfies(ordering: Ordering, relation: Relation) -> bool {
    match relation {
        Relation::Eq => ordering == Ordering::Equal,
        Relation::Gt => ordering == Ordering::Greater,
        Relation::Gte => matches!(ordering, Ordering::Greater | Ordering::Equal),
        Relation::Lt => ordering == Ordering::Less,
        Relation::Lte => matches!(ordering, Ordering::Less | Ordering::Equal),
    }
}

fn signed_limit(name: &str, value: &str) -> Result<i128, Error> {
    value.parse().map_err(|_| invalid_limit(name, value, "int"))
}

fn unsigned_limit(name: &str, value: &str) -> Result<u128, Error> {
    value
        .parse()
        .map_err(|_| invalid_limit(name, value, "uint"))
}

fn f32_limit(name: &str, value: &str) -> Result<f64, Error> {
    value
        .parse::<f32>()
        .map(f64::from)
        .map_err(|_| invalid_limit(name, value, "f32"))
}

fn f64_limit(name: &str, value: &str) -> Result<f64, Error> {
    value.parse().map_err(|_| invalid_limit(name, value, "f64"))
}

fn invalid_limit(name: &str, value: &str, expected: &str) -> Error {
    Error::InvalidRuleExpression {
        expression: format!("{name}={value}"),
        reason: format!("parameter '{name}' must be a valid {expected}"),
    }
}

fn invalid_kind(name: &str, kind: Kind) -> Error {
    Error::InvalidRuleExpression {
        expression: name.to_owned(),
        reason: format!("field kind {kind:?} does not support this comparison"),
    }
}

fn invalid_time_parameter(name: &str, value: &str) -> Error {
    Error::InvalidRuleExpression {
        expression: format!("{name}={value}"),
        reason: "SystemTime comparison does not support literal parameters; use lt/lte/gt/gte without a value or compare two fields with *_field".to_owned(),
    }
}

fn invalid_time_equality(rule: &str) -> Error {
    Error::InvalidRuleExpression {
        expression: rule.to_owned(),
        reason: "SystemTime eq/ne against the current time is unsupported; use eq_field/ne_field for time equality".to_owned(),
    }
}
