mod access;
mod cache;
mod context;
mod error;
mod field;
mod group;
mod namespace;
mod params;
mod raw_params;
mod registry;
mod selection;
mod signature;
mod spec;
mod value;

pub use access::{Access, FieldRef};
#[doc(hidden)]
pub use access::{ItemVisitor, Items, Projection, Resolve, Segment};
#[doc(hidden)]
pub use context::Context;
pub use error::{Error, FieldError};
pub use field::Field;
pub use namespace::Namespace;
pub use params::{Param, Params};
pub use signature::Signature;
pub use value::{FloatKind, IntKind, Kind, Number, UintKind, Value};

pub(crate) use cache::{CAPACITY, Cache};
pub(crate) use error::FieldErrorParts;
pub(crate) use group::{Flow, Group};
pub(crate) use raw_params::RawParam;
#[doc(hidden)]
pub use raw_params::RawParams;
pub(crate) use registry::{Entry, Registry};
pub(crate) use selection::{Fields, Selection};
#[doc(hidden)]
pub use spec::Spec;
pub(crate) use spec::{Expr, parse_expression};

pub trait Rule: Send + Sync {
    fn signature(&self) -> Signature {
        Signature::none()
    }

    /// Validates parameter semantics without deciding whether field data passes.
    fn validate_params(&self, _field: &Field<'_>) -> Result<(), Error> {
        Ok(())
    }

    fn validates_none(&self) -> bool {
        false
    }

    fn check(&self, field: &Field<'_>) -> Result<bool, Error>;
}
