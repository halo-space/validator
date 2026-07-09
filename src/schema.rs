use std::collections::BTreeMap;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};

use serde_json::Value as JsonValue;

use crate::core::{Aliases, Context, Expr, Group, Rules, Spec, parse_expression};
use crate::{Error, FieldError, FieldTarget, Kind, Params, Value, field_error, field_param};

static NEXT_ID: AtomicU64 = AtomicU64::new(1);

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
        let value = serde_yaml::from_str::<JsonValue>(yaml.as_ref()).map_err(|error| {
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

    pub(crate) fn compile(&self, rules: &Rules, aliases: &Aliases) -> Result<Tree, Error> {
        Tree::compile(&self.inner.fields, rules, aliases)
    }

    fn from_value(value: JsonValue) -> Result<Self, Error> {
        let fields = required_object(&value, "fields")?
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
struct FieldDef {
    ty: Option<Type>,
    exprs: Vec<Expr>,
    fields: BTreeMap<String, FieldDef>,
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
        let fields = object
            .get("fields")
            .map(|fields| parse_fields(name, fields))
            .transpose()?
            .unwrap_or_default();
        let ty = object
            .get("type")
            .map(|value| Type::from_value(name, value))
            .transpose()?
            .or({
                if fields.is_empty() {
                    None
                } else {
                    Some(Type::Object)
                }
            });
        let exprs = object
            .get("rules")
            .map(|rules| parse_rules(name, rules))
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
            "bool" | "boolean" => Ok(Self::Bool),
            "int" | "integer" => Ok(Self::Int),
            "uint" => Ok(Self::Uint),
            "float" | "number" => Ok(Self::Float),
            "array" => Ok(Self::Array),
            "object" | "map" => Ok(Self::Object),
            _ => Err(invalid(format!(
                "field '{field}' has unsupported type '{name}'"
            ))),
        }
    }

    fn name(self) -> &'static str {
        match self {
            Self::String => "string",
            Self::Bool => "boolean",
            Self::Int => "integer",
            Self::Uint => "uint",
            Self::Float => "number",
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
            Self::Float => value.as_f64().is_some(),
            Self::Array => value.is_array(),
            Self::Object => value.is_object(),
        }
    }
}

pub(crate) struct Tree {
    fields: BTreeMap<String, Node>,
}

impl Tree {
    fn compile(
        fields: &BTreeMap<String, FieldDef>,
        rules: &Rules,
        aliases: &Aliases,
    ) -> Result<Self, Error> {
        let fields = fields
            .iter()
            .map(|(name, field)| Ok((name.clone(), Node::compile(field, fields, rules, aliases)?)))
            .collect::<Result<BTreeMap<_, _>, Error>>()?;

        Ok(Self { fields })
    }

    pub(crate) fn validate(
        &self,
        context: &Context,
        errors: &mut Vec<FieldError>,
        data: &JsonValue,
    ) -> Result<(), Error> {
        let Some(object) = data.as_object() else {
            let mut params = Params::new();
            params.insert("expected", "object");
            errors.push(field_error(
                FieldTarget::value(),
                Kind::Other,
                "type",
                "type",
                params,
            ));
            return Ok(());
        };

        validate_fields(context, errors, "", &self.fields, object)
    }
}

struct Node {
    ty: Option<Type>,
    group: Group,
    fields: BTreeMap<String, Node>,
}

impl Node {
    fn compile(
        field: &FieldDef,
        siblings: &BTreeMap<String, FieldDef>,
        rules: &Rules,
        aliases: &Aliases,
    ) -> Result<Self, Error> {
        ensure_field_targets(&field.exprs, siblings)?;
        let group = Group::compile_with_fields(&field.exprs, rules, aliases)?;
        let children = &field.fields;
        let fields = children
            .iter()
            .map(|(name, field)| {
                Ok((
                    name.clone(),
                    Self::compile(field, children, rules, aliases)?,
                ))
            })
            .collect::<Result<BTreeMap<_, _>, Error>>()?;

        Ok(Self {
            ty: field.ty,
            group,
            fields,
        })
    }
}

fn ensure_field_targets(exprs: &[Expr], fields: &BTreeMap<String, FieldDef>) -> Result<(), Error> {
    for expr in exprs {
        if let Some(spec) = expr.single() {
            ensure_field_target(spec, fields)?;
            continue;
        }

        if let Some(alternatives) = expr.alternatives() {
            for spec in alternatives {
                ensure_field_target(spec, fields)?;
            }
        }
    }

    Ok(())
}

fn ensure_field_target(spec: &Spec, fields: &BTreeMap<String, FieldDef>) -> Result<(), Error> {
    if !crate::is_field_rule(spec.name()) {
        return Ok(());
    }

    let Some(compare) = field_param(spec.params()) else {
        return Err(invalid(format!(
            "field rule '{}' must define compare target",
            spec.name()
        )));
    };

    if fields.contains_key(compare) {
        Ok(())
    } else {
        Err(invalid(format!(
            "field rule '{}' references undeclared field '{}'",
            spec.name(),
            compare
        )))
    }
}

fn validate_fields(
    context: &Context,
    errors: &mut Vec<FieldError>,
    parent: &str,
    fields: &BTreeMap<String, Node>,
    object: &serde_json::Map<String, JsonValue>,
) -> Result<(), Error> {
    for (name, field) in fields {
        let target = FieldTarget::schema_field(parent, name);
        let value = object.get(name).unwrap_or(&JsonValue::Null);

        if value.is_null() {
            field
                .group
                .execute_with_fields(errors, target, value, context, |compare| {
                    object.get(compare).map(|value| value as &dyn Value)
                })?;
            continue;
        }

        if let Some(ty) = field.ty
            && !ty.matches(value)
        {
            let mut params = Params::new();
            params.insert("expected", ty.name());
            errors.push(field_error(target, value.kind(), "type", "type", params));
            continue;
        }

        field
            .group
            .execute_with_fields(errors, target.clone(), value, context, |compare| {
                object.get(compare).map(|value| value as &dyn Value)
            })?;

        if !field.fields.is_empty()
            && let Some(child) = value.as_object()
        {
            let parent = namespace(parent, name);
            validate_fields(context, errors, &parent, &field.fields, child)?;
        }
    }

    Ok(())
}

fn parse_fields(parent: &str, value: &JsonValue) -> Result<BTreeMap<String, FieldDef>, Error> {
    let object = value
        .as_object()
        .ok_or_else(|| invalid(format!("field '{parent}' nested fields must be an object")))?;

    object
        .iter()
        .map(|(name, field)| Ok((name.clone(), FieldDef::from_value(name, field)?)))
        .collect()
}

fn parse_rules(field: &str, value: &JsonValue) -> Result<Vec<Expr>, Error> {
    match value {
        JsonValue::Array(rules) => rules
            .iter()
            .map(|rule| parse_rule_item(field, rule))
            .collect::<Result<Vec<_>, _>>()
            .map(|exprs| exprs.into_iter().flatten().collect()),
        JsonValue::String(expr) => parse_expression(expr),
        _ => Err(invalid(format!(
            "field '{field}' rules must be a string or array"
        ))),
    }
}

fn parse_rule_item(field: &str, value: &JsonValue) -> Result<Vec<Expr>, Error> {
    match value {
        JsonValue::String(expr) => parse_expression(expr),
        JsonValue::Object(object) => {
            if object.len() != 1 {
                return Err(invalid(format!(
                    "field '{field}' rule object must contain exactly one rule"
                )));
            }

            let (name, params) = object
                .iter()
                .next()
                .expect("rule object must contain one entry");
            Ok(vec![Expr::Single(parse_rule_object(name, params)?)])
        }
        _ => Err(invalid(format!(
            "field '{field}' rule item must be a string or object"
        ))),
    }
}

fn parse_rule_object(name: &str, params: &JsonValue) -> Result<Spec, Error> {
    if params.is_null() {
        return Ok(Spec::new(name));
    }

    if let Some(object) = params.as_object() {
        let mut spec = Spec::new(name);
        for (key, value) in object {
            spec = spec.param(key, param_value(value)?);
        }
        return Ok(spec);
    }

    Ok(Spec::new(name).param(default_param_name(name), param_value(params)?))
}

fn param_value(value: &JsonValue) -> Result<String, Error> {
    match value {
        JsonValue::String(value) => Ok(value.clone()),
        JsonValue::Number(value) => Ok(value.to_string()),
        JsonValue::Bool(value) => Ok(value.to_string()),
        JsonValue::Array(values) => values
            .iter()
            .map(param_value)
            .collect::<Result<Vec<_>, _>>()
            .map(|values| values.join(",")),
        JsonValue::Null | JsonValue::Object(_) => Err(invalid(
            "rule parameters must be strings, numbers, booleans, or arrays",
        )),
    }
}

fn default_param_name(rule: &str) -> &'static str {
    match rule {
        "min" => "min",
        "max" => "max",
        "regex" => "pattern",
        "oneof" => "values",
        "eq_field" | "ne_field" | "gt_field" | "gte_field" | "lt_field" | "lte_field"
        | "fieldcontains" | "fieldexcludes" => "compare",
        _ => "value",
    }
}

fn required_object<'a>(
    value: &'a JsonValue,
    field: &'static str,
) -> Result<&'a serde_json::Map<String, JsonValue>, Error> {
    value
        .get(field)
        .and_then(JsonValue::as_object)
        .ok_or_else(|| invalid(format!("schema must contain object '{field}'")))
}

fn namespace(parent: &str, field: &str) -> String {
    if parent.is_empty() {
        field.to_owned()
    } else {
        format!("{parent}.{field}")
    }
}

fn invalid(reason: impl Into<String>) -> Error {
    Error::InvalidSchema {
        reason: reason.into(),
    }
}

#[cfg(test)]
mod tests {
    use super::Schema;

    #[test]
    fn cloned_schema_keeps_identity() {
        let schema = Schema::from_yaml(
            r#"
fields:
  title:
    type: string
"#,
        )
        .unwrap();
        let cloned = schema.clone();

        assert_eq!(schema.id(), cloned.id());
    }
}
