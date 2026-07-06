use super::{Args, Namespace, Value};

pub struct Field<'a> {
    namespace: &'a Namespace,
    struct_namespace: &'a Namespace,
    field: &'a str,
    struct_field: &'a str,
    args: &'a Args,
    value: &'a dyn Value,
}

impl<'a> Field<'a> {
    pub fn new(
        namespace: &'a Namespace,
        struct_namespace: &'a Namespace,
        field: &'a str,
        struct_field: &'a str,
        args: &'a Args,
        value: &'a dyn Value,
    ) -> Self {
        Self {
            namespace,
            struct_namespace,
            field,
            struct_field,
            args,
            value,
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

    pub fn args(&self) -> &Args {
        self.args
    }

    pub fn value(&self) -> &dyn Value {
        self.value
    }
}
