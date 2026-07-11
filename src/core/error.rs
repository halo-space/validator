use std::fmt;

use thiserror::Error as ThisError;

use super::{Kind, Namespace, Params};

#[derive(Clone, Debug, Eq, PartialEq)]
/// Describes one field that failed validation.
pub struct FieldError {
    namespace: Namespace,
    struct_namespace: Namespace,
    field: String,
    struct_field: String,
    kind: Kind,
    rule: String,
    reason: String,
    params: Params,
}

pub(crate) struct FieldErrorParts {
    pub(crate) namespace: Namespace,
    pub(crate) struct_namespace: Namespace,
    pub(crate) field: String,
    pub(crate) struct_field: String,
    pub(crate) kind: Kind,
    pub(crate) rule: String,
    pub(crate) reason: String,
    pub(crate) params: Params,
}

impl FieldError {
    pub(crate) fn new(parts: FieldErrorParts) -> Self {
        Self {
            namespace: parts.namespace,
            struct_namespace: parts.struct_namespace,
            field: parts.field,
            struct_field: parts.struct_field,
            kind: parts.kind,
            rule: parts.rule,
            reason: parts.reason,
            params: parts.params,
        }
    }

    /// Returns the runtime namespace, including nested collection positions.
    pub fn namespace(&self) -> &Namespace {
        &self.namespace
    }

    /// Returns the namespace based on Rust struct field names.
    pub fn struct_namespace(&self) -> &Namespace {
        &self.struct_namespace
    }

    /// Returns the runtime field name.
    pub fn field(&self) -> &str {
        &self.field
    }

    /// Returns the Rust struct field name.
    pub fn struct_field(&self) -> &str {
        &self.struct_field
    }

    /// Returns the field's declared or runtime value kind.
    pub fn kind(&self) -> Kind {
        self.kind
    }

    /// Returns the displayed rule name, including an outer alias when present.
    pub fn rule(&self) -> &str {
        &self.rule
    }

    /// Returns the underlying rule that caused the failure.
    pub fn reason(&self) -> &str {
        &self.reason
    }

    /// Returns the parameters bound to the failed rule.
    pub fn params(&self) -> &Params {
        &self.params
    }
}

impl fmt::Display for FieldError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "field '{}' failed on '{}' rule",
            self.namespace, self.rule
        )
    }
}

#[derive(Debug, ThisError)]
/// Errors produced while configuring or executing validation.
pub enum Error {
    /// A custom rule name contains unsupported characters.
    #[error("invalid rule name '{name}'")]
    InvalidRuleName {
        /// The rejected rule name.
        name: String,
    },
    /// An alias name contains unsupported characters.
    #[error("invalid alias name '{name}'")]
    InvalidAliasName {
        /// The rejected alias name.
        name: String,
    },
    /// A rule, alias, or reserved control name is already registered.
    #[error("validation name '{name}' is already registered")]
    DuplicateName {
        /// The duplicated name.
        name: String,
    },
    /// A rule expression could not be parsed or bound to its signature.
    #[error("invalid rule expression '{expression}': {reason}")]
    InvalidRuleExpression {
        /// The rejected expression or rule name.
        expression: String,
        /// The configuration error detail.
        reason: String,
    },
    /// A rule expression references an unregistered rule.
    #[error("unknown rule '{name}'")]
    UnknownRule {
        /// The missing rule name.
        name: String,
    },
    /// A selector or struct-level error references an undeclared field.
    #[error("unknown field '{field}'")]
    UnknownField {
        /// The unmatched relative field path.
        field: String,
    },
    /// A field-aware rule was used without sibling field access.
    #[error("rule '{name}' requires field context")]
    MissingFieldContext {
        /// The field-aware rule name.
        name: String,
    },
    /// Alias expansion encountered a cycle.
    #[error("recursive alias '{name}'")]
    RecursiveAlias {
        /// The alias participating in the cycle.
        name: String,
    },
    /// Input conversion or locale resource parsing failed.
    #[error("invalid data: {reason}")]
    InvalidData {
        /// The data error detail.
        reason: String,
    },
    /// A dynamic schema is malformed or internally inconsistent.
    #[error("invalid schema: {reason}")]
    InvalidSchema {
        /// The schema error detail.
        reason: String,
    },
    /// Schema validation was requested without a configured schema.
    #[error("schema is required for Schema validation")]
    MissingSchema,
    /// One or more fields failed validation.
    #[error("validation failed")]
    Failed(Vec<FieldError>),
}

impl Error {
    /// Creates a validation failure from field errors.
    pub fn failed(fields: Vec<FieldError>) -> Self {
        Self::Failed(fields)
    }

    /// Returns whether this error is a field validation failure.
    pub fn is_failed(&self) -> bool {
        matches!(self, Self::Failed(_))
    }

    /// Borrows the field errors when this is [`Error::Failed`].
    pub fn fields(&self) -> Option<&[FieldError]> {
        match self {
            Self::Failed(fields) => Some(fields),
            _ => None,
        }
    }

    /// Consumes this error and returns its field errors when available.
    pub fn into_fields(self) -> Option<Vec<FieldError>> {
        match self {
            Self::Failed(fields) => Some(fields),
            _ => None,
        }
    }
}
