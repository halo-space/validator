use serde_json::Value as JsonValue;

use crate::core::{Access as CoreAccess, FieldRef};

use super::compile::Scope;
use super::path::resolve;
use super::value::Field;

pub(super) struct Object<'a> {
    fields: Box<[(&'a str, Field<'a>)]>,
    paths: Box<[(&'a str, Field<'a>)]>,
}

impl<'a> Object<'a> {
    pub(super) fn new(scope: &'a Scope, object: &'a serde_json::Map<String, JsonValue>) -> Self {
        let fields = scope
            .fields
            .iter()
            .map(|(name, field)| (name.as_str(), Field::new(object.get(name), field.ty)))
            .collect::<Vec<_>>()
            .into_boxed_slice();
        let paths = scope
            .paths
            .iter()
            .map(|(name, path)| {
                (
                    name.as_str(),
                    Field::new(resolve(object, &path.segments), path.ty),
                )
            })
            .collect::<Vec<_>>()
            .into_boxed_slice();
        Self { fields, paths }
    }

    pub(super) fn get(&self, name: &str) -> Option<&Field<'a>> {
        find(&self.paths, name).or_else(|| find(&self.fields, name))
    }
}

impl CoreAccess for Object<'_> {
    fn field<'a>(&'a self, name: &'a str) -> Option<FieldRef<'a>> {
        find(&self.paths, name)
            .or_else(|| find(&self.fields, name))
            .map(|value| FieldRef::new(name, value))
    }
}

fn find<'a, 'b>(values: &'b [(&str, Field<'a>)], name: &str) -> Option<&'b Field<'a>> {
    values
        .binary_search_by(|(candidate, _)| candidate.cmp(&name))
        .ok()
        .map(|index| &values[index].1)
}
