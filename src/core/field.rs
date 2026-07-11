use std::time::SystemTime;

use super::{Access, Context, Items, Namespace, Params, Value};

/// The value and context supplied to a [`crate::Rule`].
pub struct Field<'a> {
    namespace: &'a Namespace,
    struct_namespace: &'a Namespace,
    field: &'a str,
    struct_field: &'a str,
    params: &'a Params,
    value: &'a dyn Value,
    access: Option<&'a dyn Access>,
    items: Option<&'a dyn Items>,
    now: SystemTime,
}

impl<'a> Field<'a> {
    /// Creates a field context for custom validation execution.
    pub fn new(
        namespace: &'a Namespace,
        struct_namespace: &'a Namespace,
        field: &'a str,
        struct_field: &'a str,
        params: &'a Params,
        value: &'a dyn Value,
    ) -> Self {
        Self {
            namespace,
            struct_namespace,
            field,
            struct_field,
            params,
            value,
            access: None,
            items: None,
            now: SystemTime::now(),
        }
    }

    pub(crate) fn with_context(
        mut self,
        context: &Context<'_>,
        access: Option<&'a dyn Access>,
        items: Option<&'a dyn Items>,
    ) -> Self {
        self.access = access;
        self.items = items;
        self.now = context.now();
        self
    }

    /// Returns the runtime namespace for this field.
    pub fn namespace(&self) -> &Namespace {
        self.namespace
    }

    /// Returns the namespace based on Rust struct field names.
    pub fn struct_namespace(&self) -> &Namespace {
        self.struct_namespace
    }

    /// Returns the runtime field name.
    pub fn field(&self) -> &str {
        self.field
    }

    /// Returns the Rust struct field name.
    pub fn struct_field(&self) -> &str {
        self.struct_field
    }

    /// Returns the parameters bound by the rule signature.
    pub fn params(&self) -> &Params {
        self.params
    }

    /// Returns the value being validated.
    pub fn value(&self) -> &dyn Value {
        self.value
    }

    /// Returns a declared sibling or nested target value.
    pub fn sibling<'b>(&'b self, name: &'b str) -> Option<&'b dyn Value> {
        self.access
            .and_then(|access| access.field(name))
            .map(|field| field.value())
    }

    /// Returns the captured time shared by the current validation run.
    pub fn now(&self) -> SystemTime {
        self.now
    }

    pub(crate) fn items(&self) -> Option<&dyn Items> {
        self.items
    }
}
