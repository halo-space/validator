mod noneof;
mod oneof;

use crate::{Field, Kind};

pub(super) use noneof::NoneOf;
pub(super) use oneof::OneOf;

fn contains(field: &Field<'_>) -> Option<bool> {
    let values = field
        .params()
        .get("values")
        .or_else(|| field.params().get("value"))?;
    let mut candidates = candidates(values).peekable();

    candidates.peek()?;

    match field.value().kind() {
        Kind::String => {
            let value = field.value().string()?;
            Some(candidates.any(|candidate| candidate == value.as_ref()))
        }
        Kind::Int(_) => {
            let value = field.value().int()?;
            candidates
                .map(|candidate| candidate.parse::<i128>())
                .try_fold(false, |matched, candidate| {
                    candidate.map(|candidate| matched || candidate == value)
                })
                .ok()
        }
        Kind::Uint(_) => {
            let value = field.value().uint()?;
            candidates
                .map(|candidate| candidate.parse::<u128>())
                .try_fold(false, |matched, candidate| {
                    candidate.map(|candidate| matched || candidate == value)
                })
                .ok()
        }
        Kind::Bool
        | Kind::Float(_)
        | Kind::Vec
        | Kind::Array
        | Kind::Slice
        | Kind::Map
        | Kind::Option
        | Kind::Time
        | Kind::Other => None,
    }
}

fn candidates(values: &str) -> impl Iterator<Item = &str> {
    values
        .split(',')
        .map(str::trim)
        .filter(|value| !value.is_empty())
}
