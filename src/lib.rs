//! Typed validation for Rust structs, individual values, and dynamic schemas.
//!
//! Start with [`Validate`] and [`Validator`] for derive-based validation. Use
//! [`Schema`] for YAML/JSON-driven validation, [`Rule`] for custom rules, and
//! [`Value`] for custom value types.
#![warn(missing_docs)]

extern crate self as validator;

mod core;
mod engine;
/// Localized rendering for validation failures.
pub mod i18n;
mod rules;
mod schema;
mod target;
mod traits;
/// Struct-level validation error builders used by `#[validate(check = "...")]`.
pub mod valid;

pub use self::core::{
    Error, Field, FieldError, FloatKind, IntKind, Kind, Namespace, Number, Param, Params, Rule,
    Signature, UintKind, Value,
};
pub use self::engine::Validator;
pub use self::schema::Schema;
pub(crate) use self::target::field_error;
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

/// Common validator types and traits for glob imports.
pub mod prelude {
    pub use crate::{
        Error, Field, FieldError, FloatKind, IntKind, Kind, Namespace, Number, Param, Params, Rule,
        Schema, Signature, UintKind, Validate, Validator, Value,
    };
}
