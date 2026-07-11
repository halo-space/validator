use std::time::SystemTime;

use super::{Access, Context, Items, Namespace, Params, Value};

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

    pub fn sibling<'b>(&'b self, name: &'b str) -> Option<&'b dyn Value> {
        self.access
            .and_then(|access| access.field(name))
            .map(|field| field.value())
    }

    pub fn now(&self) -> SystemTime {
        self.now
    }

    pub(crate) fn items(&self) -> Option<&dyn Items> {
        self.items
    }
}
