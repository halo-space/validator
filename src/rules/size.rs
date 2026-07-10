mod length;
mod max;
mod min;
mod range;

use super::compare::{Relation, satisfies};
pub(super) use length::Length;
pub(super) use max::Max;
pub(super) use min::Min;
pub(super) use range::Range;

fn validate(field: &crate::Field<'_>, name: &str) -> Result<(), crate::Error> {
    super::compare::validate_satisfies(field, name)
}

fn validate_bounds(field: &crate::Field<'_>, rule: &str) -> Result<(), crate::Error> {
    super::compare::validate_bounds(field, rule)
}
