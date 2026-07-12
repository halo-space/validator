mod unique;

use std::borrow::Cow;
use std::collections::hash_map::Entry;
use std::collections::{HashMap, HashSet};
use std::hash::{BuildHasher, Hash, RandomState};
use std::time::SystemTime;

use crate::core::Items;
use crate::{Error, Kind, Value};

pub(super) use unique::Unique;

#[derive(Eq, Hash, PartialEq)]
enum Part<'a> {
    None,
    String(Cow<'a, str>),
    Bool(bool),
    Int(i128),
    Uint(u128),
    Float(u64),
    Time(SystemTime),
}

enum Positions {
    One(usize),
    Many(Vec<usize>),
}

impl Positions {
    fn contains(&self, parts: &[Part<'_>], start: usize, width: usize) -> bool {
        match self {
            Self::One(position) => parts[*position..*position + width] == parts[start..],
            Self::Many(positions) => positions
                .iter()
                .any(|position| parts[*position..*position + width] == parts[start..]),
        }
    }

    fn push(&mut self, start: usize) {
        match self {
            Self::One(position) => {
                let first = *position;
                *self = Self::Many(vec![first, start]);
            }
            Self::Many(positions) => positions.push(start),
        }
    }
}

pub(crate) fn values_are_unique<'a>(
    items: impl IntoIterator<Item = &'a dyn Value>,
) -> Result<bool, Error> {
    let items = items.into_iter();
    let mut seen = HashSet::with_capacity(items.size_hint().0);

    for item in items {
        match part(Some(item)) {
            Ok(Some(part)) => {
                if !seen.insert(part) {
                    return Ok(false);
                }
            }
            Ok(None) => {}
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

pub(crate) fn fields_are_unique(items: &dyn Items, fields: &[String]) -> Result<bool, Error> {
    if fields.is_empty() {
        return Err(Error::InvalidRuleExpression {
            expression: "unique".to_owned(),
            reason: "parameter 'fields' cannot be empty".to_owned(),
        });
    }
    if fields.len() == 1 {
        return field_is_unique(items, fields);
    }

    let width = fields.len();
    let capacity = items.len();
    let mut parts = Vec::with_capacity(capacity.saturating_mul(width));
    let mut seen = HashMap::with_capacity(capacity);
    let hashes = RandomState::new();
    let mut unique = true;
    let mut invalid: Option<(String, Kind)> = None;

    items.visit(fields, &mut |values| {
        let start = parts.len();
        let mut distinct = false;

        for field in fields {
            let Some(value) = values.next() else {
                invalid = Some((field.clone(), Kind::Other));
                return false;
            };
            match part(value) {
                Ok(Some(part)) => parts.push(part),
                Ok(None) => distinct = true,
                Err(kind) => {
                    invalid = Some((field.clone(), kind));
                    return false;
                }
            }
        }

        if values.next().is_some() {
            invalid = Some(("<row>".to_owned(), Kind::Other));
            return false;
        }
        if distinct {
            parts.truncate(start);
            return true;
        }

        let hash = hashes.hash_one(&parts[start..]);
        match seen.entry(hash) {
            Entry::Vacant(entry) => {
                entry.insert(Positions::One(start));
            }
            Entry::Occupied(mut entry) => {
                if entry.get().contains(&parts, start, width) {
                    unique = false;
                    return false;
                }
                entry.get_mut().push(start);
            }
        }
        true
    })?;

    if let Some((field, kind)) = invalid {
        return Err(Error::InvalidRuleExpression {
            expression: "unique".to_owned(),
            reason: format!("field '{field}' with kind {kind:?} cannot be used as a unique key"),
        });
    }

    Ok(unique)
}

fn field_is_unique(items: &dyn Items, fields: &[String]) -> Result<bool, Error> {
    let field = fields
        .first()
        .expect("single-field unique requires one field");
    let mut seen = HashSet::with_capacity(items.len());
    let mut unique = true;
    let mut invalid = None;

    items.visit(fields, &mut |values| {
        let Some(value) = values.next() else {
            invalid = Some(Kind::Other);
            return false;
        };
        if values.next().is_some() {
            invalid = Some(Kind::Other);
            return false;
        }

        match part(value) {
            Ok(Some(part)) => {
                if !seen.insert(part) {
                    unique = false;
                    return false;
                }
            }
            Ok(None) => {}
            Err(kind) => {
                invalid = Some(kind);
                return false;
            }
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

fn part<'a>(value: Option<&'a dyn Value>) -> Result<Option<Part<'a>>, Kind> {
    let Some(value) = value else {
        return Ok(Some(Part::None));
    };

    if value.is_none() {
        return Ok(Some(Part::None));
    }

    if matches!(value.kind(), Kind::Float(_)) && value.float().is_some_and(f64::is_nan) {
        return Ok(None);
    }

    let part = match value.kind() {
        Kind::String => value.string().map(Part::String),
        Kind::Bool => value.boolean().map(Part::Bool),
        Kind::Int(_) => value.int().map(Part::Int),
        Kind::Uint(_) => value.uint().map(Part::Uint),
        Kind::Float(_) => value.float().map(float_key).map(Part::Float),
        Kind::Time => value.time().map(Part::Time),
        Kind::Vec | Kind::Array | Kind::Slice | Kind::Map | Kind::Option | Kind::Other => None,
    };

    part.ok_or_else(|| value.kind()).map(Some)
}

fn float_key(value: f64) -> u64 {
    let value = if value == 0.0 { 0.0 } else { value };

    value.to_bits()
}
