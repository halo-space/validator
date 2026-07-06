mod gt;
mod gte;
mod lt;
mod lte;

use std::cmp::Ordering;

use crate::{Field, FloatKind, Kind};

pub(super) use gt::Gt;
pub(super) use gte::Gte;
pub(super) use lt::Lt;
pub(super) use lte::Lte;

#[derive(Clone, Copy, Debug)]
pub(super) enum Relation {
    Eq,
    Gt,
    Gte,
    Lt,
    Lte,
}

pub(super) fn satisfies(field: &Field<'_>, limit_name: &str, relation: Relation) -> bool {
    field
        .args()
        .get(limit_name)
        .or_else(|| field.args().get("value"))
        .is_some_and(|limit| value_satisfies(field, limit, relation))
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
