use crate::core::FieldErrorParts;
use crate::{Error, FieldError, Kind, Namespace, Params};

/// Collects field errors from a struct-level validation function.
pub struct Valid<'a> {
    type_name: &'a str,
    errors: &'a mut Vec<FieldError>,
    kind: &'a dyn Fn(&str) -> Option<Kind>,
    invalid: Option<Error>,
}

impl<'a> Valid<'a> {
    #[doc(hidden)]
    pub fn new(
        type_name: &'a str,
        errors: &'a mut Vec<FieldError>,
        kind: &'a dyn Fn(&str) -> Option<Kind>,
    ) -> Self {
        Self {
            type_name,
            errors,
            kind,
            invalid: None,
        }
    }

    /// Starts building an error for a relative field path.
    pub fn field<'b>(&'b mut self, field: impl Into<String>) -> FieldBuilder<'a, 'b> {
        FieldBuilder {
            valid: self,
            field: field.into(),
        }
    }

    #[doc(hidden)]
    pub fn finish(&mut self) -> Result<(), Error> {
        self.invalid.take().map_or(Ok(()), Err)
    }
}

/// Selects the failed rule for a struct-level field error.
pub struct FieldBuilder<'a, 'b> {
    valid: &'b mut Valid<'a>,
    field: String,
}

impl<'a, 'b> FieldBuilder<'a, 'b> {
    /// Sets the rule name reported by this field error.
    pub fn rule(self, rule: impl Into<String>) -> ErrorBuilder<'a, 'b> {
        ErrorBuilder {
            valid: self.valid,
            field: self.field,
            rule: rule.into(),
            params: Params::new(),
        }
    }
}

/// Builds one struct-level field error and its parameters.
pub struct ErrorBuilder<'a, 'b> {
    valid: &'b mut Valid<'a>,
    field: String,
    rule: String,
    params: Params,
}

impl ErrorBuilder<'_, '_> {
    /// Adds the conventional `compare` field parameter.
    pub fn compare(mut self, field: impl Into<String>) -> Self {
        self.params.insert("compare", field);
        self
    }

    /// Adds one text parameter to the field error.
    pub fn param(mut self, name: impl Into<String>, value: impl Into<String>) -> Self {
        self.params.insert(name, value);
        self
    }

    /// Appends the completed field error to the current validation result.
    pub fn push(self) {
        if self.valid.invalid.is_some() {
            return;
        }
        let Some(kind) = (self.valid.kind)(&self.field) else {
            self.valid.invalid = Some(Error::UnknownField { field: self.field });
            return;
        };

        let namespace = format!("{}.{}", self.valid.type_name, self.field);
        self.valid.errors.push(FieldError::new(FieldErrorParts {
            namespace: Namespace::new(namespace.clone()),
            struct_namespace: Namespace::new(namespace),
            field: self.field.clone(),
            struct_field: self.field,
            kind,
            rule: self.rule.clone(),
            reason: self.rule,
            params: self.params,
        }));
    }
}
