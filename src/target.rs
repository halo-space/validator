use std::borrow::Cow;
use std::fmt;

use crate::core::FieldErrorParts;
use crate::{Error, FieldError, Kind, Namespace, Params};

#[derive(Clone)]
#[doc(hidden)]
pub struct FieldTarget<'a> {
    pub(crate) type_name: Cow<'a, str>,
    pub(crate) field_name: Cow<'a, str>,
    pub(crate) struct_field_name: Cow<'a, str>,
}

impl<'a> FieldTarget<'a> {
    pub fn new(type_name: &'a str, field_name: &'a str, struct_field_name: &'a str) -> Self {
        Self {
            type_name: Cow::Borrowed(type_name),
            field_name: Cow::Borrowed(field_name),
            struct_field_name: Cow::Borrowed(struct_field_name),
        }
    }

    pub fn index(&self, index: usize) -> Self {
        Self {
            type_name: self.type_name.clone(),
            field_name: Cow::Owned(format!("{}[{index}]", self.field_name)),
            struct_field_name: Cow::Owned(format!("{}[{index}]", self.struct_field_name)),
        }
    }

    pub fn key<K: fmt::Display>(&self, key: K) -> Self {
        let key = serde_json::to_string(&key.to_string())
            .expect("serializing a map key string must not fail");
        Self {
            type_name: self.type_name.clone(),
            field_name: Cow::Owned(format!("{}[{key}]", self.field_name)),
            struct_field_name: Cow::Owned(format!("{}[{key}]", self.struct_field_name)),
        }
    }

    #[doc(hidden)]
    pub fn struct_field_name(&self) -> &str {
        self.struct_field_name.as_ref()
    }

    pub(crate) fn value() -> Self {
        Self {
            type_name: Cow::Borrowed(""),
            field_name: Cow::Borrowed("$value"),
            struct_field_name: Cow::Borrowed("$value"),
        }
    }

    pub(crate) fn schema(field_name: impl Into<String>) -> Self {
        let field_name = field_name.into();
        Self {
            type_name: Cow::Borrowed(""),
            struct_field_name: Cow::Owned(field_name.clone()),
            field_name: Cow::Owned(field_name),
        }
    }

    pub(crate) fn schema_field(parent: &str, field_name: &str) -> Self {
        Self {
            type_name: Cow::Owned(parent.to_owned()),
            field_name: Cow::Owned(field_name.to_owned()),
            struct_field_name: Cow::Owned(field_name.to_owned()),
        }
    }
}

pub(crate) fn field_error(
    target: FieldTarget<'_>,
    kind: Kind,
    rule: &str,
    reason: &str,
    params: Params,
) -> FieldError {
    let namespace = namespace_for(&target.type_name, &target.field_name);
    let struct_namespace = namespace_for(&target.type_name, &target.struct_field_name);

    FieldError::new(FieldErrorParts {
        namespace: Namespace::new(namespace),
        struct_namespace: Namespace::new(struct_namespace),
        field: target.field_name.into_owned(),
        struct_field: target.struct_field_name.into_owned(),
        kind,
        rule: rule.to_owned(),
        reason: reason.to_owned(),
        params,
    })
}

pub(crate) fn push_nested_errors(
    errors: &mut Vec<FieldError>,
    target: FieldTarget<'_>,
    nested: Error,
) {
    if let Some(fields) = nested.into_fields() {
        for error in fields {
            errors.push(nested_field_error(target.clone(), error));
        }
    }
}

fn nested_field_error(target: FieldTarget<'_>, error: FieldError) -> FieldError {
    let parent_namespace = namespace_for(&target.type_name, &target.field_name);
    let parent_struct_namespace = namespace_for(&target.type_name, &target.struct_field_name);
    let namespace = nested_namespace(&parent_namespace, error.namespace().as_str());
    let struct_namespace =
        nested_namespace(&parent_struct_namespace, error.struct_namespace().as_str());

    FieldError::new(FieldErrorParts {
        namespace: Namespace::new(namespace),
        struct_namespace: Namespace::new(struct_namespace),
        field: error.field().to_owned(),
        struct_field: error.struct_field().to_owned(),
        kind: error.kind(),
        rule: error.rule().to_owned(),
        reason: error.reason().to_owned(),
        params: error.params().clone(),
    })
}

fn nested_namespace(parent: &str, child: &str) -> String {
    let relative = child
        .split_once('.')
        .map(|(_, relative)| relative)
        .unwrap_or(child);

    if relative.is_empty() {
        parent.to_owned()
    } else {
        format!("{parent}.{relative}")
    }
}

pub(crate) fn namespace_for(type_name: &str, field_name: &str) -> String {
    if type_name.is_empty() {
        field_name.to_owned()
    } else {
        format!("{type_name}.{field_name}")
    }
}
