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
    let limit = field
        .params()
        .get(limit_name)
        .or_else(|| field.params().get("value"));

    if field.value().kind() == Kind::Time {
        if let Some(limit) = limit {
            return Err(invalid_time_parameter(limit_name, limit));
        }

        return Ok(limit_name == "value" && time_satisfies_now(field, relation));
    }

    Ok(limit.is_some_and(|limit| value_satisfies(field, limit, relation)))
}

pub(super) fn equals(field: &Field<'_>) -> Result<bool, Error> {
    equality(field, "eq").map(|value| value.unwrap_or(false))
}

pub(super) fn not_equals(field: &Field<'_>) -> Result<bool, Error> {
    equality(field, "ne").map(|value| value.is_some_and(|equal| !equal))
}

fn equality(field: &Field<'_>, rule: &str) -> Result<Option<bool>, Error> {
    if field.value().kind() == Kind::Time {
        if let Some(value) = field.params().get("value") {
            return Err(invalid_time_parameter("value", value));
        }
        return Err(invalid_time_equality(rule));
    }

    Ok(field
        .params()
        .get("value")
        .and_then(|value| value_equals(field, value)))
}

fn value_equals(field: &Field<'_>, limit: &str) -> Option<bool> {
    match field.value().kind() {
        Kind::String => field.value().string().map(|value| value == limit),
        Kind::Bool => limit
            .parse::<bool>()
            .ok()
            .and_then(|limit| field.value().boolean().map(|value| value == limit)),
        Kind::Vec | Kind::Array | Kind::Slice | Kind::Map => {
            let limit = signed_limit(limit)?;
            field.value().len().map(|length| length as i128 == limit)
        }
        Kind::Int(_) => {
            let limit = signed_limit(limit)?;
            field.value().int().map(|value| value == limit)
        }
        Kind::Uint(_) => {
            let limit = unsigned_limit(limit)?;
            field.value().uint().map(|value| value == limit)
        }
        Kind::Float(FloatKind::F32) => {
            let limit = f32_limit(limit)?;
            field.value().float().map(|value| value == limit)
        }
        Kind::Float(FloatKind::F64) => {
            let limit = f64_limit(limit)?;
            field.value().float().map(|value| value == limit)
        }
        Kind::Option | Kind::Time | Kind::Other => None,
    }
}

fn value_satisfies(field: &Field<'_>, limit: &str, relation: Relation) -> bool {
    match field.value().kind() {
        Kind::String => {
            let Some(limit) = signed_limit(limit) else {
                return false;
            };
            field
                .value()
                .len()
                .is_some_and(|length| signed_satisfies(length as i128, limit, relation))
        }
        Kind::Vec | Kind::Array | Kind::Slice | Kind::Map => {
            let Some(limit) = signed_limit(limit) else {
                return false;
            };
            field
                .value()
                .len()
                .is_some_and(|length| signed_satisfies(length as i128, limit, relation))
        }
        Kind::Int(_) => {
            let Some(limit) = signed_limit(limit) else {
                return false;
            };
            field
                .value()
                .int()
                .is_some_and(|value| signed_satisfies(value, limit, relation))
        }
        Kind::Uint(_) => {
            let Some(limit) = unsigned_limit(limit) else {
                return false;
            };
            field
                .value()
                .uint()
                .is_some_and(|value| unsigned_satisfies(value, limit, relation))
        }
        Kind::Float(FloatKind::F32) => {
            let Some(limit) = f32_limit(limit) else {
                return false;
            };
            field
                .value()
                .float()
                .is_some_and(|value| float_satisfies(value, limit, relation))
        }
        Kind::Float(FloatKind::F64) => {
            let Some(limit) = f64_limit(limit) else {
                return false;
            };
            field
                .value()
                .float()
                .is_some_and(|value| float_satisfies(value, limit, relation))
        }
        Kind::Bool | Kind::Option | Kind::Time | Kind::Other => false,
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

fn signed_limit(value: &str) -> Option<i128> {
    value.parse().ok()
}

fn unsigned_limit(value: &str) -> Option<u128> {
    value.parse().ok()
}

fn f32_limit(value: &str) -> Option<f64> {
    value.parse::<f32>().ok().map(f64::from)
}

fn f64_limit(value: &str) -> Option<f64> {
    value.parse().ok()
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
