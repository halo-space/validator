use std::borrow::Cow;
use std::collections::BTreeMap;

use serde_json::Value as JsonValue;

use crate::Error;
use crate::core::{Access, FieldRef, ItemVisitor, Items, Kind, Value};

use super::compile::Scope;
use super::path::resolve;
use super::{Type, invalid};

pub(super) struct SchemaValue<'a> {
    value: Option<&'a JsonValue>,
    ty: Option<Type>,
}

impl<'a> SchemaValue<'a> {
    fn new(value: Option<&'a JsonValue>, ty: Option<Type>) -> Self {
        Self { value, ty }
    }

    pub(super) fn raw(&self) -> Option<&'a JsonValue> {
        self.value
    }
}

impl Value for SchemaValue<'_> {
    fn kind(&self) -> Kind {
        self.ty
            .map(Type::kind)
            .or_else(|| self.value.map(Value::kind))
            .unwrap_or(Kind::Option)
    }

    fn is_none(&self) -> bool {
        self.value.is_none_or(JsonValue::is_null)
    }

    fn required(&self) -> bool {
        self.value.is_some_and(Value::required)
    }

    fn string(&self) -> Option<Cow<'_, str>> {
        self.value.and_then(Value::string)
    }

    fn len(&self) -> Option<usize> {
        self.value.and_then(Value::len)
    }

    fn int(&self) -> Option<i128> {
        self.value.and_then(Value::int)
    }

    fn uint(&self) -> Option<u128> {
        self.value.and_then(Value::uint)
    }

    fn float(&self) -> Option<f64> {
        self.value.and_then(Value::float)
    }

    fn boolean(&self) -> Option<bool> {
        self.value.and_then(Value::boolean)
    }

    fn array_items(&self) -> Option<Box<dyn Iterator<Item = &dyn Value> + '_>> {
        self.value.and_then(Value::array_items)
    }

    fn map_values(&self) -> Option<Box<dyn Iterator<Item = &dyn Value> + '_>> {
        self.value.and_then(Value::map_values)
    }
}

pub(super) struct SchemaAccess<'a> {
    fields: BTreeMap<&'a str, SchemaValue<'a>>,
    paths: BTreeMap<&'a str, SchemaValue<'a>>,
}

impl<'a> SchemaAccess<'a> {
    pub(super) fn new(scope: &'a Scope, object: &'a serde_json::Map<String, JsonValue>) -> Self {
        let fields = scope
            .fields
            .iter()
            .map(|(name, field)| (name.as_str(), SchemaValue::new(object.get(name), field.ty)))
            .collect::<BTreeMap<_, _>>();
        let paths = scope
            .paths
            .values()
            .map(|path| {
                (
                    path.name.as_str(),
                    SchemaValue::new(resolve(object, &path.segments), path.ty),
                )
            })
            .collect();
        Self { fields, paths }
    }

    pub(super) fn get(&self, name: &str) -> Option<&SchemaValue<'a>> {
        self.paths.get(name).or_else(|| self.fields.get(name))
    }
}

impl Access for SchemaAccess<'_> {
    fn field<'a>(&'a self, name: &'a str) -> Option<FieldRef<'a>> {
        self.paths
            .get(name)
            .or_else(|| self.fields.get(name))
            .map(|value| FieldRef::new(name, value))
    }
}

pub(super) enum Projected {
    Missing,
    Value,
    Invalid,
}

enum JsonItem<'a> {
    Object {
        raw: &'a serde_json::Map<String, JsonValue>,
        access: SchemaAccess<'a>,
    },
    Invalid,
}

pub(super) struct JsonItems<'a> {
    values: Vec<JsonItem<'a>>,
    scope: &'a Scope,
}

impl<'a> JsonItems<'a> {
    pub(super) fn new(values: &'a [JsonValue], scope: &'a Scope) -> Self {
        let values = values
            .iter()
            .map(|value| {
                if let Some(object) = value.as_object() {
                    JsonItem::Object {
                        raw: object,
                        access: SchemaAccess::new(scope, object),
                    }
                } else {
                    JsonItem::Invalid
                }
            })
            .collect();
        Self { values, scope }
    }
}

impl Items for JsonItems<'_> {
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
                JsonItem::Invalid => continue,
                JsonItem::Object { raw, access } => (raw, access),
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
                (!value.is_none()).then_some(value as &dyn Value)
            });
            if !visitor(&mut values) {
                break;
            }
        }
        Ok(())
    }
}
