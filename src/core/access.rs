#![allow(missing_docs)]

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
pub struct Segment<'a, T: ?Sized>(&'a T);

impl<'a, T: ?Sized> Segment<'a, T> {
    pub fn new(value: &'a T) -> Self {
        Self(value)
    }
}

#[doc(hidden)]
pub trait Resolve<'a> {
    type Target: ?Sized;

    fn resolve(self) -> Option<&'a Self::Target>;
}

impl<'a, T> Resolve<'a> for Segment<'a, Option<T>> {
    type Target = T;

    fn resolve(self) -> Option<&'a Self::Target> {
        self.0.as_ref()
    }
}

impl<'a, T: ?Sized> Resolve<'a> for &Segment<'a, T> {
    type Target = T;

    fn resolve(self) -> Option<&'a Self::Target> {
        Some(self.0)
    }
}

#[doc(hidden)]
pub trait Items {
    fn visit<'a>(&'a self, fields: &[String], visitor: &mut ItemVisitor<'a>) -> Result<(), Error>;
}

pub type ItemVisitor<'a> =
    dyn for<'b> FnMut(&'b mut (dyn Iterator<Item = Option<&'a dyn Value>> + 'b)) -> bool + 'a;

#[doc(hidden)]
pub struct Projection<'a, T> {
    items: &'a [T],
    fields: &'static [&'static str],
    kind: Kind,
    project: for<'b> fn(&'b T, &str) -> Option<&'b dyn Value>,
}

impl<'a, T> Projection<'a, T> {
    pub fn new(
        items: &'a [T],
        fields: &'static [&'static str],
        kind: Kind,
        project: for<'b> fn(&'b T, &str) -> Option<&'b dyn Value>,
    ) -> Self {
        Self {
            items,
            fields,
            kind,
            project,
        }
    }
}

impl<T> Value for Projection<'_, T> {
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

impl<T> Items for Projection<'_, T> {
    fn visit<'a>(&'a self, fields: &[String], visitor: &mut ItemVisitor<'a>) -> Result<(), Error> {
        if fields.len() != self.fields.len()
            || fields
                .iter()
                .zip(self.fields)
                .any(|(actual, expected)| actual != expected)
        {
            return Err(Error::InvalidRuleExpression {
                expression: "unique".to_owned(),
                reason: "projected fields do not match the compiled item access".to_owned(),
            });
        }

        for item in self.items {
            let mut values = fields.iter().map(|field| (self.project)(item, field));
            if !visitor(&mut values) {
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
