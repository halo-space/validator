use crate::core::FieldErrorParts;
use crate::{FieldError, Kind, Namespace, Params};

pub struct Valid<'a> {
    type_name: &'a str,
    errors: &'a mut Vec<FieldError>,
    kind: &'a dyn Fn(&str) -> Kind,
}

impl<'a> Valid<'a> {
    #[doc(hidden)]
    pub fn new(
        type_name: &'a str,
        errors: &'a mut Vec<FieldError>,
        kind: &'a dyn Fn(&str) -> Kind,
    ) -> Self {
        Self {
            type_name,
            errors,
            kind,
        }
    }

    pub fn field<'b>(&'b mut self, field: impl Into<String>) -> FieldBuilder<'a, 'b> {
        FieldBuilder {
            valid: self,
            field: field.into(),
        }
    }
}

pub struct FieldBuilder<'a, 'b> {
    valid: &'b mut Valid<'a>,
    field: String,
}

impl<'a, 'b> FieldBuilder<'a, 'b> {
    pub fn rule(self, rule: impl Into<String>) -> ErrorBuilder<'a, 'b> {
        ErrorBuilder {
            valid: self.valid,
            field: self.field,
            rule: rule.into(),
            params: Params::new(),
        }
    }
}

pub struct ErrorBuilder<'a, 'b> {
    valid: &'b mut Valid<'a>,
    field: String,
    rule: String,
    params: Params,
}

impl ErrorBuilder<'_, '_> {
    pub fn compare(mut self, field: impl Into<String>) -> Self {
        self.params.insert("compare", field);
        self
    }

    pub fn param(mut self, name: impl Into<String>, value: impl Into<String>) -> Self {
        self.params.insert(name, value);
        self
    }

    pub fn push(self) {
        let namespace = format!("{}.{}", self.valid.type_name, self.field);
        let kind = (self.valid.kind)(&self.field);
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
