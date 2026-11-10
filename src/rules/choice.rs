mod none_of;
mod none_of_ci;
mod one_of;
mod one_of_ci;

use crate::{Field, Kind};

pub(super) use none_of::NoneOf;
pub(super) use none_of_ci::NoneOfCi;
pub(super) use one_of::OneOf;
pub(super) use one_of_ci::OneOfCi;

fn validate(field: &Field<'_>, rule: &str) -> Result<(), crate::Error> {
    let candidates = field
        .params()
        .list("values")
        .ok_or_else(|| invalid(rule, "rule requires 'values' parameters"))?;

    match field.value().kind() {
        Kind::Int(_) => {
            for candidate in candidates {
                candidate.parse::<i128>().map_err(|_| {
                    invalid(rule, format!("candidate '{candidate}' must be a valid int"))
                })?;
            }
        }
        Kind::Uint(_) => {
            for candidate in candidates {
                candidate.parse::<u128>().map_err(|_| {
                    invalid(
                        rule,
                        format!("candidate '{candidate}' must be a valid uint"),
                    )
                })?;
            }
        }
        Kind::String
        | Kind::Bool
        | Kind::Float(_)
        | Kind::Vec
        | Kind::Array
        | Kind::Slice
        | Kind::Map
        | Kind::Option
        | Kind::Time
        | Kind::Other => {}
    }

    Ok(())
}

fn contains(field: &Field<'_>, rule: &str) -> Result<Option<bool>, crate::Error> {
    validate(field, rule)?;
    let candidates = field
        .params()
        .list("values")
        .ok_or_else(|| invalid(rule, "rule requires 'values' parameters"))?;

    match field.value().kind() {
        Kind::String => Ok(field.value().string().map(|value| {
            candidates
                .iter()
                .any(|candidate| candidate == value.as_ref())
        })),
        Kind::Int(_) => {
            let Some(value) = field.value().int() else {
                return Ok(None);
            };
            Ok(Some(candidates.iter().any(|candidate| {
                candidate
                    .parse::<i128>()
                    .is_ok_and(|candidate| candidate == value)
            })))
        }
        Kind::Uint(_) => {
            let Some(value) = field.value().uint() else {
                return Ok(None);
            };
            Ok(Some(candidates.iter().any(|candidate| {
                candidate
                    .parse::<u128>()
                    .is_ok_and(|candidate| candidate == value)
            })))
        }
        Kind::Bool
        | Kind::Float(_)
        | Kind::Vec
        | Kind::Array
        | Kind::Slice
        | Kind::Map
        | Kind::Option
        | Kind::Time
        | Kind::Other => Ok(None),
    }
}

fn contains_ignore_case(field: &Field<'_>) -> Option<bool> {
    let mut candidates = field
        .params()
        .list("values")?
        .iter()
        .map(String::as_str)
        .peekable();

    candidates.peek()?;

    let value = field.value().string()?.to_lowercase();
    Some(candidates.any(|candidate| candidate.to_lowercase() == value))
}

fn invalid(rule: &str, reason: impl Into<String>) -> crate::Error {
    crate::Error::InvalidRuleExpression {
        expression: rule.to_owned(),
        reason: reason.into(),
    }
}
