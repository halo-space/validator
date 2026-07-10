use std::collections::BTreeMap;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};

use serde_json::Value as JsonValue;

use crate::core::{Context, Entry, Expr, Group, Registry, Spec, parse_expression};
use crate::{Error, FieldError, FieldTarget, Kind, Params, Value, field_error};

static NEXT_ID: AtomicU64 = AtomicU64::new(1);
pub(crate) const TYPE_RULE: &str = "type";

#[cfg(test)]
pub(crate) fn internal_rule_names() -> impl Iterator<Item = &'static str> {
    [TYPE_RULE].into_iter()
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
        reject_unknown_keys(
            object,
            &["type", "rules", "fields"],
            &format!("field '{name}'"),
        )?;
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
            "boolean" => Ok(Self::Bool),
            "integer" => Ok(Self::Int),
            "uint" => Ok(Self::Uint),
            "number" => Ok(Self::Float),
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
    fn compile(fields: &BTreeMap<String, FieldDef>, registry: &Registry) -> Result<Self, Error> {
        let fields = fields
            .iter()
            .map(|(name, field)| Ok((name.clone(), Node::compile(field, fields, registry)?)))
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
                TYPE_RULE,
                TYPE_RULE,
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
        registry: &Registry,
    ) -> Result<Self, Error> {
        ensure_field_targets(&field.exprs, siblings, registry, &mut Vec::new())?;
        let group = Group::compile_with_fields(&field.exprs, registry)?;
        let children = &field.fields;
        let fields = children
            .iter()
            .map(|(name, field)| Ok((name.clone(), Self::compile(field, children, registry)?)))
            .collect::<Result<BTreeMap<_, _>, Error>>()?;

        Ok(Self {
            ty: field.ty,
            group,
            fields,
        })
    }
}

fn ensure_field_targets(
    exprs: &[Expr],
    fields: &BTreeMap<String, FieldDef>,
    registry: &Registry,
    aliases: &mut Vec<String>,
) -> Result<(), Error> {
    for expr in exprs {
        if let Some(spec) = expr.single() {
            ensure_field_target(spec, fields, registry, aliases)?;
            continue;
        }

        if let Some(alternatives) = expr.alternatives() {
            for spec in alternatives {
                ensure_field_target(spec, fields, registry, aliases)?;
            }
        }
    }

    Ok(())
}

fn ensure_field_target(
    spec: &Spec,
    fields: &BTreeMap<String, FieldDef>,
    registry: &Registry,
    aliases: &mut Vec<String>,
) -> Result<(), Error> {
    let params = match registry.get(spec.name()) {
        Some(Entry::Rule(rule)) if rule.signature().requires_fields() => {
            rule.signature().bind(spec.name(), spec.params())?
        }
        Some(Entry::Rule(_)) | None => return Ok(()),
        Some(Entry::Alias(exprs)) => {
            if aliases.iter().any(|name| name == spec.name()) {
                return Err(Error::RecursiveAlias {
                    name: spec.name().to_owned(),
                });
            }
            aliases.push(spec.name().to_owned());
            ensure_field_targets(exprs, fields, registry, aliases)?;
            aliases.pop();
            return Ok(());
        }
    };
    let targets = params
        .text("compare")
        .into_iter()
        .chain(
            params
                .list("fields")
                .into_iter()
                .flatten()
                .map(String::as_str),
        )
        .chain(
            params
                .pairs("conditions")
                .into_iter()
                .flatten()
                .map(|(name, _)| name.as_str()),
        )
        .collect::<Vec<_>>();
    if targets.is_empty() {
        return Err(invalid(format!(
            "field rule '{}' must define target field",
            spec.name()
        )));
    }

    for target in targets {
        if !fields.contains_key(target) {
            return Err(invalid(format!(
                "field rule '{}' references undeclared field '{}'",
                spec.name(),
                target
            )));
        }
    }

    Ok(())
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
                .execute_with_fields(errors, target, value, context, object)?;
            continue;
        }

        if let Some(ty) = field.ty
            && !ty.matches(value)
        {
            let mut params = Params::new();
            params.insert("expected", ty.name());
            errors.push(field_error(
                target,
                value.kind(),
                TYPE_RULE,
                TYPE_RULE,
                params,
            ));
            continue;
        }

        field
            .group
            .execute_with_fields(errors, target.clone(), value, context, object)?;

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
            spec = match value {
                JsonValue::Array(values) => spec.named_list(key, param_list(values)?),
                _ => spec.named(key, param_value(value)?),
            };
        }
        return Ok(spec);
    }

    match params {
        JsonValue::Array(values) => {
            let mut spec = Spec::new(name);
            for value in param_list(values)? {
                spec = spec.positional(value);
            }
            Ok(spec)
        }
        _ => Ok(Spec::new(name).positional(param_value(params)?)),
    }
}

fn param_value(value: &JsonValue) -> Result<String, Error> {
    match value {
        JsonValue::String(value) => Ok(value.clone()),
        JsonValue::Number(value) => Ok(value.to_string()),
        JsonValue::Bool(value) => Ok(value.to_string()),
        JsonValue::Null | JsonValue::Array(_) | JsonValue::Object(_) => Err(invalid(
            "rule parameter must be a string, number, or boolean",
        )),
    }
}

fn param_list(values: &[JsonValue]) -> Result<Vec<String>, Error> {
    values.iter().map(param_value).collect()
}

fn reject_unknown_keys(
    object: &serde_json::Map<String, JsonValue>,
    allowed: &[&str],
    context: &str,
) -> Result<(), Error> {
    if let Some(key) = object.keys().find(|key| !allowed.contains(&key.as_str())) {
        return Err(invalid(format!("{context} uses unknown key '{key}'")));
    }

    Ok(())
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
