use std::fmt;

use thiserror::Error as ThisError;

use super::{Args, Namespace};

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FieldError {
    namespace: Namespace,
    struct_namespace: Namespace,
    field: String,
    struct_field: String,
    rule: String,
    actual_rule: String,
    args: Args,
}

impl FieldError {
    pub fn new(
        namespace: Namespace,
        struct_namespace: Namespace,
        field: impl Into<String>,
        struct_field: impl Into<String>,
        rule: impl Into<String>,
        actual_rule: impl Into<String>,
        args: Args,
    ) -> Self {
        Self {
            namespace,
            struct_namespace,
            field: field.into(),
            struct_field: struct_field.into(),
            rule: rule.into(),
            actual_rule: actual_rule.into(),
            args,
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

    pub fn rule(&self) -> &str {
        &self.rule
    }

    pub fn actual_rule(&self) -> &str {
        &self.actual_rule
    }

    pub fn args(&self) -> &Args {
        &self.args
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

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct Errors {
    fields: Vec<FieldError>,
}

impl Errors {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn push(&mut self, error: FieldError) {
        self.fields.push(error);
    }

    pub fn is_empty(&self) -> bool {
        self.fields.is_empty()
    }

    pub fn len(&self) -> usize {
        self.fields.len()
    }

    pub fn iter(&self) -> impl Iterator<Item = &FieldError> {
        self.fields.iter()
    }

    pub fn into_vec(self) -> Vec<FieldError> {
        self.fields
    }
}

impl fmt::Display for Errors {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for (index, error) in self.fields.iter().enumerate() {
            if index > 0 {
                writeln!(f)?;
            }
            write!(f, "{error}")?;
        }
        Ok(())
    }
}

impl std::error::Error for Errors {}

#[derive(Debug, ThisError)]
pub enum Error {
    #[error("invalid rule name '{name}'")]
    InvalidRuleName { name: String },
    #[error("invalid alias name '{name}'")]
    InvalidAliasName { name: String },
    #[error("invalid alias '{name}': {reason}")]
    InvalidAlias { name: String, reason: String },
    #[error("unknown rule '{name}'")]
    UnknownRule { name: String },
    #[error("unknown alias '{name}'")]
    UnknownAlias { name: String },
}
