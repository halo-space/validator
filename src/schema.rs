mod parser;

use std::borrow::Cow;
use std::collections::BTreeMap;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};

use serde_json::Value as JsonValue;

use crate::core::{
    Access, Context, Entry, Expr, FieldRef, Group, ItemVisitor, Items, Registry, Spec,
};
use crate::{
    Error, FieldError, FieldTarget, FloatKind, IntKind, Kind, Params, UintKind, Value, field_error,
};

use self::parser::{fields as parse_fields, invalid, reject_unknown_keys, rules as parse_rules};

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
    root: Scope,
}

impl Tree {
    fn compile(fields: &BTreeMap<String, FieldDef>, registry: &Registry) -> Result<Self, Error> {
        let tree = Self {
            root: Scope::compile(fields, registry)?,
        };
        preflight_fields(&Context::new(), "", &tree.root)?;
        Ok(tree)
    }

    pub(crate) fn validate(
        &self,
        context: &Context<'_>,
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

        validate_fields(context, errors, "", &self.root, object)
    }
}

struct Scope {
    fields: BTreeMap<String, Node>,
    paths: BTreeMap<String, Path>,
}

impl Scope {
    fn compile(fields: &BTreeMap<String, FieldDef>, registry: &Registry) -> Result<Self, Error> {
        Self::compile_with_paths(fields, registry, BTreeMap::new())
    }

    fn compile_with_paths(
        fields: &BTreeMap<String, FieldDef>,
        registry: &Registry,
        mut paths: BTreeMap<String, Path>,
    ) -> Result<Self, Error> {
        let fields = fields
            .iter()
            .map(|(name, field)| {
                Ok((
                    name.clone(),
                    Node::compile(field, fields, registry, &mut paths)?,
                ))
            })
            .collect::<Result<BTreeMap<_, _>, Error>>()?;

        Ok(Self { fields, paths })
    }

    fn projected(
        &self,
        object: &serde_json::Map<String, JsonValue>,
        name: &str,
    ) -> Result<Projected, Error> {
        let path = self
            .paths
            .get(name)
            .ok_or_else(|| invalid(format!("undeclared array item field path '{name}'")))?;
        let mut scope = self;
        let mut object = object;

        for (index, segment) in path.segments.iter().enumerate() {
            let node = scope
                .fields
                .get(segment)
                .ok_or_else(|| invalid(format!("undeclared array item field path '{name}'")))?;
            let Some(value) = object.get(segment) else {
                return Ok(Projected::Missing);
            };
            if value.is_null() {
                return Ok(Projected::Missing);
            }
            if index + 1 == path.segments.len() {
                return Ok(if node.ty.is_none_or(|ty| ty.matches(value)) {
                    Projected::Value
                } else {
                    Projected::Invalid
                });
            }

            let Some(next) = value.as_object() else {
                return Ok(Projected::Invalid);
            };
            let Fields::Declared(children) = &node.children else {
                return Ok(Projected::Invalid);
            };
            scope = children;
            object = next;
        }

        Ok(Projected::Missing)
    }
}

struct Node {
    ty: Option<Type>,
    group: Group,
    children: Fields<Scope>,
}

impl Node {
    fn compile(
        field: &FieldDef,
        siblings: &BTreeMap<String, FieldDef>,
        registry: &Registry,
        paths: &mut BTreeMap<String, Path>,
    ) -> Result<Self, Error> {
        let item_fields = if field.ty == Some(Type::Array) {
            field.fields.declared()
        } else {
            None
        };
        let mut item_paths = BTreeMap::new();
        ensure_field_targets(
            &field.exprs,
            siblings,
            item_fields,
            registry,
            &mut Vec::new(),
            paths,
            &mut item_paths,
        )?;
        let group = if item_fields.is_some() {
            Group::compile_with_fields_and_items(&field.exprs, registry)?
        } else {
            Group::compile_with_fields(&field.exprs, registry)?
        };
        let children = match &field.fields {
            Fields::Absent => Fields::Absent,
            Fields::Declared(fields) => {
                Fields::Declared(Scope::compile_with_paths(fields, registry, item_paths)?)
            }
        };

        Ok(Self {
            ty: field.ty,
            group,
            children,
        })
    }
}

fn ensure_field_targets(
    exprs: &[Expr],
    fields: &BTreeMap<String, FieldDef>,
    item_fields: Option<&BTreeMap<String, FieldDef>>,
    registry: &Registry,
    aliases: &mut Vec<String>,
    paths: &mut BTreeMap<String, Path>,
    item_paths: &mut BTreeMap<String, Path>,
) -> Result<(), Error> {
    for expr in exprs {
        if let Some(spec) = expr.single() {
            ensure_field_target(
                spec,
                fields,
                item_fields,
                registry,
                aliases,
                paths,
                item_paths,
            )?;
            continue;
        }

        if let Some(alternatives) = expr.alternatives() {
            for spec in alternatives {
                ensure_field_target(
                    spec,
                    fields,
                    item_fields,
                    registry,
                    aliases,
                    paths,
                    item_paths,
                )?;
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
    paths: &mut BTreeMap<String, Path>,
    item_paths: &mut BTreeMap<String, Path>,
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
            ensure_field_targets(
                exprs,
                fields,
                item_fields,
                registry,
                aliases,
                paths,
                item_paths,
            )?;
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

        let path_target = signature.field_target(&params);
        for target in targets {
            if path_target == Some(target) {
                let path = Path::compile(spec.name(), target, fields)?;
                if path.segments.len() > 1 && fields.contains_key(target) {
                    return Err(invalid(format!(
                        "field rule '{}' path '{}' conflicts with a literal field of the same name",
                        spec.name(),
                        target
                    )));
                }
                paths.entry(target.to_owned()).or_insert(path);
            } else if !fields.contains_key(target) {
                return Err(invalid(format!(
                    "field rule '{}' references undeclared field '{}'",
                    spec.name(),
                    target
                )));
            }
        }
    }

    if let Some(targets) = signature.item_fields(&params) {
        let item_fields = item_fields.ok_or_else(|| {
            invalid(format!(
                "rule '{}' requires an array with object fields",
                spec.name()
            ))
        })?;
        for target in targets {
            let path = Path::compile(spec.name(), target, item_fields)?;
            if path.segments.len() > 1 && item_fields.contains_key(target) {
                return Err(invalid(format!(
                    "rule '{}' path '{}' conflicts with a literal item field of the same name",
                    spec.name(),
                    target
                )));
            }
            if matches!(path.ty, Some(Type::Array | Type::Object)) {
                return Err(invalid(format!(
                    "rule '{}' item field '{}' cannot be used as a unique key",
                    spec.name(),
                    target
                )));
            }
            item_paths.entry(target.to_owned()).or_insert(path);
        }
    }

    Ok(())
}

struct Path {
    name: String,
    segments: Vec<String>,
    ty: Option<Type>,
}

impl Path {
    fn compile(rule: &str, name: &str, fields: &BTreeMap<String, FieldDef>) -> Result<Self, Error> {
        if !name.contains('.') {
            let field = fields.get(name).ok_or_else(|| {
                invalid(format!(
                    "field rule '{rule}' references undeclared field '{name}'"
                ))
            })?;
            return Ok(Self {
                name: name.to_owned(),
                segments: vec![name.to_owned()],
                ty: field.ty,
            });
        }

        let segments = parse_path(rule, name)?;
        let mut fields = fields;
        let mut ty = None;

        for (index, segment) in segments.iter().enumerate() {
            let Some(field) = fields.get(segment) else {
                let reason = if segments.len() == 1 {
                    format!("field rule '{rule}' references undeclared field '{name}'")
                } else {
                    format!("field rule '{rule}' references undeclared path '{name}'")
                };
                return Err(invalid(reason));
            };
            if index + 1 == segments.len() {
                ty = field.ty;
                break;
            }
            if field.ty != Some(Type::Object) {
                let prefix = segments[..=index].join(".");
                return Err(invalid(format!(
                    "field rule '{rule}' path '{name}' requires object segment '{prefix}'"
                )));
            }
            let Some(children) = field.fields.declared() else {
                return Err(invalid(format!(
                    "field rule '{rule}' references undeclared path '{name}'"
                )));
            };
            fields = children;
        }

        Ok(Self {
            name: name.to_owned(),
            segments,
            ty,
        })
    }
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
    fields: BTreeMap<&'a str, SchemaValue<'a>>,
    paths: BTreeMap<&'a str, SchemaValue<'a>>,
}

impl<'a> SchemaAccess<'a> {
    fn new(scope: &'a Scope, object: &'a serde_json::Map<String, JsonValue>) -> Self {
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

    fn get(&self, name: &str) -> Option<&SchemaValue<'a>> {
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

enum Projected {
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

struct JsonItems<'a> {
    values: Vec<JsonItem<'a>>,
    scope: &'a Scope,
}

impl<'a> JsonItems<'a> {
    fn new(values: &'a [JsonValue], scope: &'a Scope) -> Self {
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

fn validate_fields<'a>(
    context: &Context<'_>,
    errors: &mut Vec<FieldError>,
    parent: &str,
    scope: &'a Scope,
    object: &'a serde_json::Map<String, JsonValue>,
) -> Result<(), Error> {
    let access = SchemaAccess::new(scope, object);
    for (name, field) in &scope.fields {
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
            match &field.children {
                Fields::Absent => {
                    field
                        .group
                        .run_with_fields(errors, target.clone(), value, context, &access)?;
                }
                Fields::Declared(children) => {
                    let items = JsonItems::new(values, children);
                    field.group.run_with_fields_and_items(
                        errors,
                        target.clone(),
                        value,
                        context,
                        &access,
                        &items,
                    )?;
                    validate_array_fields(context, errors, parent, name, children, values)?;
                }
            }
        } else {
            field
                .group
                .run_with_fields(errors, target.clone(), value, context, &access)?;

            if let Fields::Declared(children) = &field.children
                && let Some(object) = raw.as_object()
            {
                let parent = namespace(parent, name);
                validate_fields(context, errors, &parent, children, object)?;
            }
        }
    }

    Ok(())
}

fn preflight_fields(context: &Context<'_>, parent: &str, scope: &Scope) -> Result<(), Error> {
    let object = serde_json::Map::new();
    let access = SchemaAccess::new(scope, &object);

    for (name, field) in &scope.fields {
        let target = FieldTarget::schema_field(parent, name);
        let value = access
            .get(name)
            .expect("Schema access contains every declared field");
        field
            .group
            .validate_with_fields(target, value, context, &access)?;

        if let Fields::Declared(children) = &field.children {
            preflight_fields(context, &namespace(parent, name), children)?;
        }
    }

    Ok(())
}

fn validate_array_fields(
    context: &Context<'_>,
    errors: &mut Vec<FieldError>,
    parent: &str,
    name: &str,
    scope: &Scope,
    values: &[JsonValue],
) -> Result<(), Error> {
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

        validate_fields(context, errors, &format!("{array}[{index}]"), scope, object)?;
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

fn resolve<'a>(
    object: &'a serde_json::Map<String, JsonValue>,
    segments: &[String],
) -> Option<&'a JsonValue> {
    let mut value = object.get(segments.first()?)?;
    for segment in &segments[1..] {
        value = value.as_object()?.get(segment)?;
    }
    Some(value)
}

fn parse_path(rule: &str, path: &str) -> Result<Vec<String>, Error> {
    if path.is_empty() {
        return Err(invalid_path(rule, path));
    }

    path.split('.')
        .map(|segment| {
            if is_identifier(segment) {
                Ok(segment.to_owned())
            } else {
                Err(invalid_path(rule, path))
            }
        })
        .collect()
}

fn is_identifier(segment: &str) -> bool {
    if segment.is_empty()
        || segment.starts_with("r#")
        || matches!(segment, "_" | "Self" | "crate" | "self" | "super")
    {
        return false;
    }

    let mut chars = segment.chars();
    let Some(first) = chars.next() else {
        return false;
    };
    (first == '_' || unicode_ident::is_xid_start(first))
        && chars.all(unicode_ident::is_xid_continue)
}

fn invalid_path(rule: &str, path: &str) -> Error {
    invalid(format!(
        "field rule '{rule}' has invalid field path '{path}'; expected dot-separated Rust field identifiers"
    ))
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
