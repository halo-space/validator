use super::{Error, Kind, Value};

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

#[doc(hidden)]
pub trait Items {
    fn visit<'a>(
        &'a self,
        field: &str,
        visitor: &mut dyn FnMut(Option<&'a dyn Value>) -> bool,
    ) -> Result<(), Error>;
}

#[doc(hidden)]
pub struct Projection<'a, T, V> {
    items: &'a [T],
    field: &'a str,
    kind: Kind,
    project: for<'b> fn(&'b T) -> &'b V,
}

impl<'a, T, V> Projection<'a, T, V> {
    pub fn new(
        items: &'a [T],
        field: &'a str,
        kind: Kind,
        project: for<'b> fn(&'b T) -> &'b V,
    ) -> Self {
        Self {
            items,
            field,
            kind,
            project,
        }
    }
}

impl<T, V> Value for Projection<'_, T, V> {
    fn kind(&self) -> Kind {
        self.kind
    }

    fn required(&self) -> bool {
        !self.items.is_empty()
    }

    fn len(&self) -> Option<usize> {
        Some(self.items.len())
    }
}

impl<T, V: Value> Items for Projection<'_, T, V> {
    fn visit<'a>(
        &'a self,
        field: &str,
        visitor: &mut dyn FnMut(Option<&'a dyn Value>) -> bool,
    ) -> Result<(), Error> {
        if field != self.field {
            return Err(Error::InvalidRuleExpression {
                expression: "unique".to_owned(),
                reason: format!("projected field '{}' does not match '{field}'", self.field),
            });
        }

        for item in self.items {
            if !visitor(Some((self.project)(item))) {
                break;
            }
        }
        Ok(())
    }
}

impl Access for serde_json::Map<String, serde_json::Value> {
    fn field<'a>(&'a self, name: &'a str) -> Option<FieldRef<'a>> {
        self.get(name).map(|value| FieldRef::new(name, value))
    }
}
