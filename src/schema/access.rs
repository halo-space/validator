use std::collections::BTreeMap;

use serde_json::Value as JsonValue;

use crate::core::{Access as CoreAccess, FieldRef};

use super::compile::Scope;
use super::path::resolve;
use super::value::Field;

pub(super) struct Object<'a> {
    fields: BTreeMap<&'a str, Field<'a>>,
    paths: BTreeMap<&'a str, Field<'a>>,
}

impl<'a> Object<'a> {
    pub(super) fn new(scope: &'a Scope, object: &'a serde_json::Map<String, JsonValue>) -> Self {
        let fields = scope
            .fields
            .iter()
            .map(|(name, field)| (name.as_str(), Field::new(object.get(name), field.ty)))
            .collect::<BTreeMap<_, _>>();
        let paths = scope
            .paths
            .values()
            .map(|path| {
                (
                    path.name.as_str(),
                    Field::new(resolve(object, &path.segments), path.ty),
                )
            })
            .collect();
        Self { fields, paths }
    }

    pub(super) fn get(&self, name: &str) -> Option<&Field<'a>> {
        self.paths.get(name).or_else(|| self.fields.get(name))
    }
}

impl CoreAccess for Object<'_> {
    fn field<'a>(&'a self, name: &'a str) -> Option<FieldRef<'a>> {
        self.paths
            .get(name)
            .or_else(|| self.fields.get(name))
            .map(|value| FieldRef::new(name, value))
    }
}
