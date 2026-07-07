use std::fmt;

use thiserror::Error as ThisError;

use super::{Kind, Namespace, Params};

#[derive(Clone, Debug, Eq, PartialEq)]
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

    pub fn namespace(&self) -> &Namespace {
        &self.namespace
    }

    pub fn struct_namespace(&self) -> &Namespace {
        &self.struct_namespace
    }

    pub fn field(&self) -> &str {
        &self.field
    }

    pub fn struct_field(&self) -> &str {
        &self.struct_field
    }

    pub fn kind(&self) -> Kind {
        self.kind
    }

    pub fn rule(&self) -> &str {
        &self.rule
    }

    pub fn reason(&self) -> &str {
        &self.reason
    }

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
pub enum Error {
    #[error("invalid rule name '{name}'")]
    InvalidRuleName { name: String },
    #[error("invalid alias name '{name}'")]
    InvalidAliasName { name: String },
    #[error("invalid rule expression '{expression}': {reason}")]
    InvalidRuleExpression { expression: String, reason: String },
    #[error("unknown rule '{name}'")]
    UnknownRule { name: String },
    #[error("unknown alias '{name}'")]
    UnknownAlias { name: String },
    #[error("invalid schema: {reason}")]
    InvalidSchema { reason: String },
    #[error("schema is required for validate_map")]
    MissingSchema,
    #[error("validation failed")]
    Failed(Vec<FieldError>),
}

impl Error {
    pub fn failed(fields: Vec<FieldError>) -> Self {
        Self::Failed(fields)
    }

    pub fn is_failed(&self) -> bool {
        matches!(self, Self::Failed(_))
    }

    pub fn fields(&self) -> Option<&[FieldError]> {
        match self {
            Self::Failed(fields) => Some(fields),
            _ => None,
        }
    }

    pub fn into_fields(self) -> Option<Vec<FieldError>> {
        match self {
            Self::Failed(fields) => Some(fields),
            _ => None,
        }
    }
}
