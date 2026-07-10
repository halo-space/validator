mod noneof;
mod noneofci;
mod oneof;
mod oneofci;

use crate::{Field, Kind};

pub(super) use noneof::NoneOf;
pub(super) use noneofci::NoneOfCi;
pub(super) use oneof::OneOf;
pub(super) use oneofci::OneOfCi;

fn contains(field: &Field<'_>) -> Option<bool> {
    let mut candidates = field
        .params()
        .list("values")?
        .iter()
        .map(String::as_str)
        .peekable();

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
