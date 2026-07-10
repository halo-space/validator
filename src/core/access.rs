use super::Value;

pub struct FieldRef<'a> {
    name: &'a str,
    value: &'a dyn Value,
}

impl<'a> FieldRef<'a> {
    pub fn new<V: Value + 'a>(name: &'a str, value: &'a V) -> Self {
        Self { name, value }
    }

    pub fn name(&self) -> &str {
        self.name
    }

    pub fn value(&self) -> &'a dyn Value {
        self.value
    }
}

pub trait Access {
    fn field<'a>(&'a self, name: &'a str) -> Option<FieldRef<'a>>;
}

impl Access for serde_json::Map<String, serde_json::Value> {
    fn field<'a>(&'a self, name: &'a str) -> Option<FieldRef<'a>> {
        self.get(name).map(|value| FieldRef::new(name, value))
    }
}
