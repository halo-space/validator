use std::time::SystemTime;

use super::{Context, Namespace, Params, Value};

pub struct Field<'a> {
    namespace: &'a Namespace,
    struct_namespace: &'a Namespace,
    field: &'a str,
    struct_field: &'a str,
    params: &'a Params,
    value: &'a dyn Value,
    now: SystemTime,
}

impl<'a> Field<'a> {
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
            now: SystemTime::now(),
        }
    }

    pub(crate) fn with_context(
        namespace: &'a Namespace,
        struct_namespace: &'a Namespace,
        field: &'a str,
        struct_field: &'a str,
        params: &'a Params,
        value: &'a dyn Value,
        context: &Context,
    ) -> Self {
        Self {
            namespace,
            struct_namespace,
            field,
            struct_field,
            params,
            value,
            now: context.now(),
        }
    }

    pub fn namespace(&self) -> &Namespace {
        self.namespace
    }

    pub fn struct_namespace(&self) -> &Namespace {
        self.struct_namespace
    }

    pub fn field(&self) -> &str {
        self.field
    }

    pub fn struct_field(&self) -> &str {
        self.struct_field
    }

    pub fn params(&self) -> &Params {
        self.params
    }

    pub fn value(&self) -> &dyn Value {
        self.value
    }

    pub fn now(&self) -> SystemTime {
        self.now
    }
}
