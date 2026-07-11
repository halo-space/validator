use serde_json::Value as JsonValue;

use crate::Error;
use crate::core::{ItemVisitor, Items as CoreItems, Value as CoreValue};

use super::access::Object;
use super::compile::Scope;
use super::invalid;

pub(super) enum Projected {
    Missing,
    Value,
    Invalid,
}

enum Item<'a> {
    Object {
        raw: &'a serde_json::Map<String, JsonValue>,
        access: Object<'a>,
    },
    Invalid,
}

pub(super) struct Collection<'a> {
    values: Vec<Item<'a>>,
    scope: &'a Scope,
}

impl<'a> Collection<'a> {
    pub(super) fn new(values: &'a [JsonValue], scope: &'a Scope) -> Self {
        let values = values
            .iter()
            .map(|value| {
                if let Some(object) = value.as_object() {
                    Item::Object {
                        raw: object,
                        access: Object::new(scope, object),
                    }
                } else {
                    Item::Invalid
                }
            })
            .collect();
        Self { values, scope }
    }
}

impl CoreItems for Collection<'_> {
    fn visit<'a>(&'a self, fields: &[String], visitor: &mut ItemVisitor<'a>) -> Result<(), Error> {
        if let Some(field) = fields
            .iter()
            .find(|field| !self.scope.paths.contains_key(field.as_str()))
        {
            return Err(invalid(format!(
                "undeclared array item field path '{field}'"
            )));
        }

        for item in &self.values {
            let (raw, access) = match item {
                Item::Invalid => continue,
                Item::Object { raw, access } => (raw, access),
            };
            let mut malformed = false;
            for field in fields {
                if matches!(self.scope.projected(raw, field)?, Projected::Invalid) {
                    malformed = true;
                    break;
                }
            }
            if malformed {
                continue;
            }

            let mut values = fields.iter().map(|field| {
                let value = access
                    .get(field)
                    .expect("projected item path is compiled before validation");
                (!value.is_none()).then_some(value as &dyn CoreValue)
            });
            if !visitor(&mut values) {
                break;
            }
        }
        Ok(())
    }
}
