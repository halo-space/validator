mod access;
mod compile;
mod items;
mod parser;
mod path;
mod validate;
mod value;

#[cfg(test)]
mod tests;

use std::collections::BTreeMap;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};

use serde_json::Value as JsonValue;

use crate::core::{Expr, Registry};
use crate::{Error, FloatKind, IntKind, Kind, UintKind};

pub(crate) use self::compile::Tree;
use self::parser::{
    expressions as parse_expressions, fields as parse_fields, invalid, reject_unknown_keys,
};

static NEXT_ID: AtomicU64 = AtomicU64::new(1);
pub(crate) const TYPE_FAILURE: &str = "type";

#[cfg(test)]
pub(crate) fn internal_rule_names() -> impl Iterator<Item = &'static str> {
    [TYPE_FAILURE].into_iter()
}

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub(crate) struct SchemaId(u64);

impl SchemaId {
    fn next() -> Self {
        Self(NEXT_ID.fetch_add(1, Ordering::Relaxed))
    }
}

#[derive(Clone, Debug)]
pub struct Schema {
    inner: Arc<Inner>,
}

#[derive(Debug)]
struct Inner {
    id: SchemaId,
    fields: BTreeMap<String, FieldDef>,
}

impl Schema {
    pub fn from_yaml(yaml: impl AsRef<str>) -> Result<Self, Error> {
        let value = serde_yaml_ng::from_str::<JsonValue>(yaml.as_ref()).map_err(|error| {
            Error::InvalidSchema {
                reason: error.to_string(),
            }
        })?;

        Self::from_value(value)
    }

    pub fn from_json(json: impl AsRef<str>) -> Result<Self, Error> {
        let value = serde_json::from_str::<JsonValue>(json.as_ref()).map_err(|error| {
            Error::InvalidSchema {
                reason: error.to_string(),
            }
        })?;

        Self::from_value(value)
    }

    pub(crate) fn id(&self) -> SchemaId {
        self.inner.id
    }

    pub(crate) fn compile(&self, registry: &Registry) -> Result<Tree, Error> {
        Tree::compile(&self.inner.fields, registry)
    }

    fn from_value(value: JsonValue) -> Result<Self, Error> {
        let object = value
            .as_object()
            .ok_or_else(|| invalid("schema must be an object"))?;
        reject_unknown_keys(object, &["fields"], "schema")?;
        let fields = object
            .get("fields")
            .and_then(JsonValue::as_object)
            .ok_or_else(|| invalid("schema must contain object 'fields'"))?
            .iter()
            .map(|(name, field)| Ok((name.clone(), FieldDef::from_value(name, field)?)))
            .collect::<Result<BTreeMap<_, _>, Error>>()?;

        Ok(Self {
            inner: Arc::new(Inner {
                id: SchemaId::next(),
                fields,
            }),
        })
    }
}

#[derive(Clone, Debug)]
enum Fields<T> {
    Absent,
    Declared(T),
}

impl<T> Fields<T> {
    const fn declared(&self) -> Option<&T> {
        match self {
            Self::Absent => None,
            Self::Declared(fields) => Some(fields),
        }
    }

    const fn is_declared(&self) -> bool {
        matches!(self, Self::Declared(_))
    }
}

#[derive(Clone, Debug)]
struct FieldDef {
    ty: Option<Type>,
    exprs: Vec<Expr>,
    fields: Fields<BTreeMap<String, FieldDef>>,
}

impl FieldDef {
    fn from_value(name: &str, value: &JsonValue) -> Result<Self, Error> {
        let object = value
            .as_object()
            .ok_or_else(|| invalid(format!("field '{name}' must be an object")))?;
        if object.contains_key("types") {
            return Err(invalid(format!(
                "field '{name}' uses unsupported key 'types'; use 'type'"
            )));
        }
        reject_unknown_keys(
            object,
            &["type", "rules", "fields"],
            &format!("field '{name}'"),
        )?;
        let fields = match object.get("fields") {
            Some(fields) => Fields::Declared(parse_fields(name, fields)?),
            None => Fields::Absent,
        };
        let ty = object
            .get("type")
            .map(|value| Type::from_value(name, value))
            .transpose()?
            .or(fields.is_declared().then_some(Type::Object));
        if fields.is_declared() && !matches!(ty, Some(Type::Array | Type::Object)) {
            return Err(invalid(format!(
                "field '{name}' can define nested fields only for type 'object' or 'array'"
            )));
        }
        let exprs = object
            .get("rules")
            .map(|rules| parse_expressions(name, rules))
            .transpose()?
            .unwrap_or_default();

        Ok(Self { ty, exprs, fields })
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum Type {
    String,
    Bool,
    Int,
    Uint,
    Float,
    Array,
    Object,
}

impl Type {
    fn from_value(field: &str, value: &JsonValue) -> Result<Self, Error> {
        let Some(name) = value.as_str() else {
            return Err(invalid(format!("field '{field}' type must be a string")));
        };

        match name {
            "string" => Ok(Self::String),
            "boolean" => Ok(Self::Bool),
            "int" => Ok(Self::Int),
            "uint" => Ok(Self::Uint),
            "float" => Ok(Self::Float),
            "array" => Ok(Self::Array),
            "object" => Ok(Self::Object),
            _ => Err(invalid(format!(
                "field '{field}' has unsupported type '{name}'"
            ))),
        }
    }

    fn name(self) -> &'static str {
        match self {
            Self::String => "string",
            Self::Bool => "boolean",
            Self::Int => "int",
            Self::Uint => "uint",
            Self::Float => "float",
            Self::Array => "array",
            Self::Object => "object",
        }
    }

    fn matches(self, value: &JsonValue) -> bool {
        match self {
            Self::String => value.is_string(),
            Self::Bool => value.is_boolean(),
            Self::Int => value.as_i64().is_some(),
            Self::Uint => value.as_u64().is_some(),
            Self::Float => value.as_number().is_some_and(serde_json::Number::is_f64),
            Self::Array => value.is_array(),
            Self::Object => value.is_object(),
        }
    }

    fn kind(self) -> Kind {
        match self {
            Self::String => Kind::String,
            Self::Bool => Kind::Bool,
            Self::Int => Kind::Int(IntKind::I64),
            Self::Uint => Kind::Uint(UintKind::U64),
            Self::Float => Kind::Float(FloatKind::F64),
            Self::Array => Kind::Vec,
            Self::Object => Kind::Map,
        }
    }
}
