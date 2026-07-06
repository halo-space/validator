mod args;
mod error;
mod field;
mod namespace;
mod registry;
mod spec;
mod value;

pub use args::Args;
pub use error::{Error, Errors, FieldError};
pub use field::Field;
pub use namespace::Namespace;
pub use value::{FloatKind, IntKind, Kind, Number, UintKind, Value};

pub(crate) use registry::{Aliases, Rules};

pub trait Rule: Send + Sync {
    fn check(&self, field: &Field<'_>) -> bool;
}
