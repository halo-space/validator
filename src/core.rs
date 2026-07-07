mod access;
mod error;
mod field;
mod namespace;
mod params;
mod registry;
mod spec;
mod value;

pub use access::{Access, FieldRef};
pub use error::{Error, FieldError};
pub use field::Field;
pub use namespace::Namespace;
pub use params::Params;
pub use value::{FloatKind, IntKind, Kind, Number, UintKind, Value};

pub(crate) use error::FieldErrorParts;
pub(crate) use registry::{Aliases, Rules};
pub(crate) use spec::{RuleGroup, RuleSpec, parse_rule_expression};

pub trait Rule: Send + Sync {
    fn check(&self, field: &Field<'_>) -> bool;
}
