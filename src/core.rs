mod access;
mod context;
mod error;
mod field;
mod group;
mod namespace;
mod params;
mod registry;
mod spec;
mod value;

pub use access::{Access, FieldRef};
#[doc(hidden)]
pub use context::Context;
pub use error::{Error, FieldError};
pub use field::Field;
pub use namespace::Namespace;
pub use params::Params;
pub use value::{FloatKind, IntKind, Kind, Number, UintKind, Value};

pub(crate) use error::FieldErrorParts;
pub(crate) use group::Group;
pub(crate) use registry::{Aliases, Rules};
pub(crate) use spec::{Expr, Spec, parse_expression};

pub(crate) type Fields<'a> = dyn Fn(&str) -> Option<&'a dyn Value> + 'a;

pub trait Rule: Send + Sync {
    fn check(&self, field: &Field<'_>) -> Result<bool, Error>;
}
