//! Typed validation for Rust structs, individual values, and dynamic schemas.
//!
//! Start with [`Validate`] and [`Validator`] for derive-based validation. Use
//! [`Schema`] for YAML/JSON-driven validation, [`Rule`] for custom rules, and
//! [`Value`] for custom value types.
#![warn(missing_docs)]
#![allow(async_fn_in_trait)]

extern crate self as halo;

mod core;
/// Localized rendering for validation failures.
pub mod i18n;
mod rule;
mod rules;
mod schema;
mod target;
mod traits;
/// Struct-level validation error builders used by `#[validate(check = "...")]`.
pub mod valid;
#[path = "validator.rs"]
mod validator_impl;

pub use self::core::{
    Error, Field, FieldError, FloatKind, IntKind, Kind, Namespace, Number, Param, Params, Rule,
    Signature, UintKind, Value,
};
pub use self::rule::Provider;
pub use self::schema::Schema;
pub(crate) use self::target::field_error;
pub use self::traits::{Selective, Validate};
pub use self::validator_impl::{Config, Stats, Validator};
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
        Config, Error, Field, FieldError, FloatKind, IntKind, Kind, Namespace, Number, Param,
        Params, Provider, Rule, Schema, Signature, Stats, UintKind, Validate, Validator, Value,
    };
}

/// Public validator namespace used by the `halo::validator` entry point.
pub mod validator {
    pub use super::__private;
    pub use super::{
        Config, Error, Field, FieldError, FloatKind, IntKind, Kind, Namespace, Number, Param,
        Params, Provider, Rule, Schema, Signature, Stats, UintKind, Validate, Validator, Value,
    };
    pub use super::{i18n, valid};

    /// Common validator types for `use halo::validator::prelude::*` imports.
    pub mod prelude {
        pub use super::super::prelude::*;
    }
}
