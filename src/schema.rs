use std::borrow::Cow;
use std::collections::BTreeMap;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};

use serde_json::Value as JsonValue;

use crate::core::{
    Access, Context, Entry, Expr, FieldRef, Group, Items, Registry, Spec, parse_expression,
};
use crate::{
    Error, FieldError, FieldTarget, FloatKind, IntKind, Kind, Params, UintKind, Value, field_error,
};

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
    has_fields: bool,
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
        let has_fields = object.contains_key("fields");
        let fields = object
            .get("fields")
            .map(|fields| parse_fields(name, fields))
            .transpose()?
            .unwrap_or_default();
        let ty = object
            .get("type")
            .map(|value| Type::from_value(name, value))
            .transpose()?
            .or(has_fields.then_some(Type::Object));
        if has_fields && !matches!(ty, Some(Type::Array | Type::Object)) {
            return Err(invalid(format!(
                "field '{name}' can define nested fields only for type 'object' or 'array'"
            )));
        }
        let exprs = object
            .get("rules")
            .map(|rules| parse_rules(name, rules))
            .transpose()?
            .unwrap_or_default();

        Ok(Self {
            ty,
            exprs,
            fields,
            has_fields,
        })
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

pub(crate) struct Tree {
    fields: BTreeMap<String, Node>,
}

impl Tree {
    fn compile(fields: &BTreeMap<String, FieldDef>, registry: &Registry) -> Result<Self, Error> {
        let fields = fields
            .iter()
            .map(|(name, field)| Ok((name.clone(), Node::compile(field, fields, registry)?)))
            .collect::<Result<BTreeMap<_, _>, Error>>()?;
        let tree = Self { fields };
        preflight_fields(&Context::new(), "", &tree.fields)?;
        Ok(tree)
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
                data.kind(),
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
    has_fields: bool,
}

impl Node {
    fn compile(
        field: &FieldDef,
        siblings: &BTreeMap<String, FieldDef>,
        registry: &Registry,
    ) -> Result<Self, Error> {
        let item_fields = (field.ty == Some(Type::Array)).then_some(&field.fields);
        ensure_field_targets(
            &field.exprs,
            siblings,
            item_fields,
            registry,
            &mut Vec::new(),
        )?;
        let group = if item_fields.is_some() {
            Group::compile_with_fields_and_items(&field.exprs, registry)?
        } else {
            Group::compile_with_fields(&field.exprs, registry)?
        };
        let children = &field.fields;
        let fields = children
            .iter()
            .map(|(name, field)| Ok((name.clone(), Self::compile(field, children, registry)?)))
            .collect::<Result<BTreeMap<_, _>, Error>>()?;

        Ok(Self {
            ty: field.ty,
            group,
            fields,
            has_fields: field.has_fields,
        })
    }
}

fn ensure_field_targets(
    exprs: &[Expr],
    fields: &BTreeMap<String, FieldDef>,
    item_fields: Option<&BTreeMap<String, FieldDef>>,
    registry: &Registry,
    aliases: &mut Vec<String>,
) -> Result<(), Error> {
    for expr in exprs {
        if let Some(spec) = expr.single() {
            ensure_field_target(spec, fields, item_fields, registry, aliases)?;
            continue;
        }

        if let Some(alternatives) = expr.alternatives() {
            for spec in alternatives {
                ensure_field_target(spec, fields, item_fields, registry, aliases)?;
            }
        }
    }

    Ok(())
}

fn ensure_field_target(
    spec: &Spec,
    fields: &BTreeMap<String, FieldDef>,
    item_fields: Option<&BTreeMap<String, FieldDef>>,
    registry: &Registry,
    aliases: &mut Vec<String>,
) -> Result<(), Error> {
    let (signature, params) = match registry.get(spec.name()) {
        Some(Entry::Rule(rule)) => {
            let signature = rule.signature();
            let params = signature.bind(spec.name(), spec.params())?;
            (signature, params)
        }
        None => return Ok(()),
        Some(Entry::Alias(exprs)) => {
            if aliases.iter().any(|name| name == spec.name()) {
                return Err(Error::RecursiveAlias {
                    name: spec.name().to_owned(),
                });
            }
            aliases.push(spec.name().to_owned());
            ensure_field_targets(exprs, fields, item_fields, registry, aliases)?;
            aliases.pop();
            return Ok(());
        }
    };
    if signature.requires_fields() {
        let targets = signature.field_targets(&params);
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
    }

    if let Some(target) = signature.item_field(&params) {
        let item_fields = item_fields.ok_or_else(|| {
            invalid(format!(
                "rule '{}' requires an array with object fields",
                spec.name()
            ))
        })?;
        if !item_fields.contains_key(target) {
            return Err(invalid(format!(
                "rule '{}' references undeclared item field '{}'",
                spec.name(),
                target
            )));
        }
        if matches!(
            item_fields.get(target).and_then(|field| field.ty),
            Some(Type::Array | Type::Object)
        ) {
            return Err(invalid(format!(
                "rule '{}' item field '{}' cannot be used as a unique key",
                spec.name(),
                target
            )));
        }
    }

    Ok(())
}

struct SchemaValue<'a> {
    value: Option<&'a JsonValue>,
    ty: Option<Type>,
}

impl<'a> SchemaValue<'a> {
    fn new(value: Option<&'a JsonValue>, ty: Option<Type>) -> Self {
        Self { value, ty }
    }

    fn raw(&self) -> Option<&'a JsonValue> {
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

struct SchemaAccess<'a> {
    values: BTreeMap<&'a str, SchemaValue<'a>>,
}

impl<'a> SchemaAccess<'a> {
    fn new(
        fields: &'a BTreeMap<String, Node>,
        object: &'a serde_json::Map<String, JsonValue>,
    ) -> Self {
        let values = fields
            .iter()
            .map(|(name, field)| (name.as_str(), SchemaValue::new(object.get(name), field.ty)))
            .collect();
        Self { values }
    }

    fn get(&self, name: &str) -> Option<&SchemaValue<'a>> {
        self.values.get(name)
    }
}

impl Access for SchemaAccess<'_> {
    fn field<'a>(&'a self, name: &'a str) -> Option<FieldRef<'a>> {
        self.values
            .get(name)
            .map(|value| FieldRef::new(name, value))
    }
}

enum JsonItem<'a> {
    Null,
    Object(SchemaAccess<'a>),
    Invalid,
}

struct JsonItems<'a> {
    values: Vec<JsonItem<'a>>,
    fields: &'a BTreeMap<String, Node>,
}

impl<'a> JsonItems<'a> {
    fn new(values: &'a [JsonValue], fields: &'a BTreeMap<String, Node>) -> Self {
        let values = values
            .iter()
            .map(|value| {
                if value.is_null() {
                    JsonItem::Null
                } else if let Some(object) = value.as_object() {
                    JsonItem::Object(SchemaAccess::new(fields, object))
                } else {
                    JsonItem::Invalid
                }
            })
            .collect();
        Self { values, fields }
    }
}

impl Items for JsonItems<'_> {
    fn visit<'a>(
        &'a self,
        field: &str,
        visitor: &mut dyn FnMut(Option<&'a dyn Value>) -> bool,
    ) -> Result<(), Error> {
        if !self.fields.contains_key(field) {
            return Err(invalid(format!("undeclared array item field '{field}'")));
        }

        let definition = self
            .fields
            .get(field)
            .expect("projected field is checked above");
        for value in &self.values {
            let projected = match value {
                JsonItem::Null => None,
                JsonItem::Invalid => continue,
                JsonItem::Object(access) => {
                    let value = access.get(field).expect("projected field is checked above");
                    if let Some(raw) = value.raw()
                        && !raw.is_null()
                        && definition.ty.is_some_and(|ty| !ty.matches(raw))
                    {
                        continue;
                    }
                    (!value.is_none()).then_some(value as &dyn Value)
                }
            };
            if !visitor(projected) {
                break;
            }
        }
        Ok(())
    }
}

fn validate_fields<'a>(
    context: &Context,
    errors: &mut Vec<FieldError>,
    parent: &str,
    fields: &'a BTreeMap<String, Node>,
    object: &'a serde_json::Map<String, JsonValue>,
) -> Result<(), Error> {
    let access = SchemaAccess::new(fields, object);
    for (name, field) in fields {
        let target = FieldTarget::schema_field(parent, name);
        let value = access
            .get(name)
            .expect("Schema access contains every declared field");

        if value.is_none() {
            field
                .group
                .run_with_fields(errors, target, value, context, &access)?;
            continue;
        }

        let raw = value.raw().expect("non-null Schema value has raw data");

        if let Some(ty) = field.ty
            && !ty.matches(raw)
        {
            let mut params = Params::new();
            params.insert("expected", ty.name());
            errors.push(field_error(
                target,
                raw.kind(),
                TYPE_RULE,
                TYPE_RULE,
                params,
            ));
            continue;
        }

        if field.ty == Some(Type::Array) {
            let values = raw
                .as_array()
                .expect("array type is checked before rule execution");
            let items = JsonItems::new(values, &field.fields);
            field.group.run_with_fields_and_items(
                errors,
                target.clone(),
                value,
                context,
                &access,
                &items,
            )?;
            validate_array_fields(
                context,
                errors,
                parent,
                name,
                &field.fields,
                field.has_fields,
                values,
            )?;
        } else {
            field
                .group
                .run_with_fields(errors, target.clone(), value, context, &access)?;

            if !field.fields.is_empty()
                && let Some(child) = raw.as_object()
            {
                let parent = namespace(parent, name);
                validate_fields(context, errors, &parent, &field.fields, child)?;
            }
        }
    }

    Ok(())
}

fn preflight_fields(
    context: &Context,
    parent: &str,
    fields: &BTreeMap<String, Node>,
) -> Result<(), Error> {
    let object = serde_json::Map::new();
    let access = SchemaAccess::new(fields, &object);

    for (name, field) in fields {
        let target = FieldTarget::schema_field(parent, name);
        let value = access
            .get(name)
            .expect("Schema access contains every declared field");
        field
            .group
            .validate_with_fields(target, value, context, &access)?;

        if !field.fields.is_empty() {
            preflight_fields(context, &namespace(parent, name), &field.fields)?;
        }
    }

    Ok(())
}

fn validate_array_fields(
    context: &Context,
    errors: &mut Vec<FieldError>,
    parent: &str,
    name: &str,
    fields: &BTreeMap<String, Node>,
    has_fields: bool,
    values: &[JsonValue],
) -> Result<(), Error> {
    if !has_fields {
        return Ok(());
    }

    let array = namespace(parent, name);
    for (index, value) in values.iter().enumerate() {
        let Some(object) = value.as_object() else {
            let mut params = Params::new();
            params.insert("expected", "object");
            errors.push(field_error(
                FieldTarget::schema(format!("{array}[{index}]")),
                value.kind(),
                TYPE_RULE,
                TYPE_RULE,
                params,
            ));
            continue;
        };

        validate_fields(
            context,
            errors,
            &format!("{array}[{index}]"),
            fields,
            object,
        )?;
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

    #[test]
    fn scalar_schema_field_rejects_nested_fields() {
        let error = Schema::from_yaml(
            r#"
fields:
  title:
    type: string
    fields:
      value:
        type: string
"#,
        )
        .unwrap_err();

        assert!(matches!(error, crate::Error::InvalidSchema { .. }));
    }
}
