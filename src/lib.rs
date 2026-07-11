extern crate self as validator;

mod core;
mod engine;
pub mod i18n;
mod rules;
mod schema;
mod target;
mod traits;
pub mod valid;

pub use self::core::{
    Error, Field, FieldError, FloatKind, IntKind, Kind, Namespace, Number, Param, Params, Rule,
    Signature, UintKind, Value,
};
pub use self::engine::Validator;
pub use self::schema::Schema;
pub(crate) use self::target::{field_error, namespace_for};
pub use self::traits::{Selective, Validate};
pub use validator_derive::Validate;

#[doc(hidden)]
pub mod __private {
    pub use crate::Selective;
    pub use crate::core::{
        Access, Context, FieldRef, Projection, RawParams, Resolve, Segment, Spec,
    };
    pub use crate::target::FieldTarget;
}

pub mod prelude {
    pub use crate::{
        Error, Field, FieldError, FloatKind, IntKind, Kind, Namespace, Number, Param, Params, Rule,
        Schema, Signature, UintKind, Validate, Validator, Value,
    };
}
