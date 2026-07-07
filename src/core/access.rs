use super::Value;

pub struct FieldRef<'a> {
    name: &'a str,
    value: &'a dyn Value,
}

impl<'a> FieldRef<'a> {
    pub fn new<V: Value + 'a>(name: &'a str, value: &'a V) -> Self {
        Self { name, value }
    }

    pub fn name(&self) -> &'a str {
        self.name
    }

    pub fn value(&self) -> &'a dyn Value {
        self.value
    }
}

pub trait Access {
    fn field(&self, name: &str) -> Option<FieldRef<'_>>;
}
