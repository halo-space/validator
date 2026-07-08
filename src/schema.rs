use std::collections::BTreeMap;

use serde_json::Value as JsonValue;

use crate::core::{Context, RuleGroup, RuleSpec, parse_rule_expression};
use crate::{
    Error, FieldError, FieldTarget, Kind, Params, Validator, Value, field_error,
    is_cross_field_rule,
};

#[derive(Clone, Debug)]
pub struct Schema {
    fields: BTreeMap<String, FieldSchema>,
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

    pub(crate) fn ensure_rules(&self, validator: &Validator) -> Result<(), Error> {
        ensure_field_rules(validator, &self.fields)
    }

    pub(crate) fn validate(
        &self,
        validator: &Validator,
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

        validate_fields(validator, context, errors, "", &self.fields, object)
    }

    fn from_value(value: JsonValue) -> Result<Self, Error> {
        let fields = required_object(&value, "fields")?
            .iter()
            .map(|(name, field)| Ok((name.clone(), FieldSchema::from_value(name, field)?)))
            .collect::<Result<BTreeMap<_, _>, Error>>()?;

        Ok(Self { fields })
    }
}

#[derive(Clone, Debug)]
struct FieldSchema {
    ty: Option<SchemaType>,
    rules: Vec<RuleGroup>,
    fields: BTreeMap<String, FieldSchema>,
}

impl FieldSchema {
    fn from_value(name: &str, value: &JsonValue) -> Result<Self, Error> {
        let object = value
            .as_object()
            .ok_or_else(|| invalid_schema(format!("field '{name}' must be an object")))?;
        if object.contains_key("types") {
            return Err(invalid_schema(format!(
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
            .map(|value| SchemaType::from_value(name, value))
            .transpose()?
            .or({
                if fields.is_empty() {
                    None
                } else {
                    Some(SchemaType::Object)
                }
            });
        let rules = object
            .get("rules")
            .map(|rules| parse_rules(name, rules))
            .transpose()?
            .unwrap_or_default();

        Ok(Self { ty, rules, fields })
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum SchemaType {
    String,
    Bool,
    Int,
    Uint,
    Float,
    Array,
    Object,
}

impl SchemaType {
    fn from_value(field: &str, value: &JsonValue) -> Result<Self, Error> {
        let Some(name) = value.as_str() else {
            return Err(invalid_schema(format!(
                "field '{field}' type must be a string"
            )));
        };

        match name {
            "string" => Ok(Self::String),
            "bool" | "boolean" => Ok(Self::Bool),
            "int" | "integer" => Ok(Self::Int),
            "uint" => Ok(Self::Uint),
            "float" | "number" => Ok(Self::Float),
            "array" => Ok(Self::Array),
            "object" | "map" => Ok(Self::Object),
            _ => Err(invalid_schema(format!(
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

fn ensure_field_rules(
    validator: &Validator,
    fields: &BTreeMap<String, FieldSchema>,
) -> Result<(), Error> {
    for field in fields.values() {
        ensure_schema_rule_groups(validator, &field.rules)?;
        ensure_compare_targets(&field.rules, fields)?;
        ensure_field_rules(validator, &field.fields)?;
    }

    Ok(())
}

fn ensure_schema_rule_groups(validator: &Validator, groups: &[RuleGroup]) -> Result<(), Error> {
    for group in groups {
        if let Some(spec) = group.single() {
            ensure_schema_rule(validator, spec)?;
            continue;
        }

        if let Some(alternatives) = group.alternatives() {
            for spec in alternatives {
                ensure_schema_rule(validator, spec)?;
            }
        }
    }

    Ok(())
}

fn ensure_schema_rule(validator: &Validator, spec: &RuleSpec) -> Result<(), Error> {
    if is_cross_field_rule(spec.name()) {
        return Ok(());
    }

    validator.ensure_rule_groups(&[RuleGroup::Single(spec.clone())])
}

fn ensure_compare_targets(
    rules: &[RuleGroup],
    fields: &BTreeMap<String, FieldSchema>,
) -> Result<(), Error> {
    for group in rules {
        if let Some(spec) = group.single() {
            ensure_compare_target(spec, fields)?;
            continue;
        }

        if let Some(alternatives) = group.alternatives() {
            for spec in alternatives {
                ensure_compare_target(spec, fields)?;
            }
        }
    }

    Ok(())
}

fn ensure_compare_target(
    spec: &RuleSpec,
    fields: &BTreeMap<String, FieldSchema>,
) -> Result<(), Error> {
    if !is_cross_field_rule(spec.name()) {
        return Ok(());
    }

    let Some(compare) = compare_param(spec.params()) else {
        return Err(invalid_schema(format!(
            "cross-field rule '{}' must define compare target",
            spec.name()
        )));
    };

    if fields.contains_key(compare) {
        Ok(())
    } else {
        Err(invalid_schema(format!(
            "cross-field rule '{}' references undeclared field '{}'",
            spec.name(),
            compare
        )))
    }
}

fn validate_fields(
    validator: &Validator,
    context: &Context,
    errors: &mut Vec<FieldError>,
    parent: &str,
    fields: &BTreeMap<String, FieldSchema>,
    object: &serde_json::Map<String, JsonValue>,
) -> Result<(), Error> {
    for (name, field) in fields {
        let target = FieldTarget::schema_field(parent, name);
        let value = object.get(name).unwrap_or(&JsonValue::Null);

        if value.is_null() {
            validator.validate_rule_groups_with_compare(
                errors,
                target,
                value,
                &field.rules,
                |compare| object.get(compare).map(|value| value as &dyn Value),
                context,
            )?;
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

        validator.validate_rule_groups_with_compare(
            errors,
            target.clone(),
            value,
            &field.rules,
            |compare| object.get(compare).map(|value| value as &dyn Value),
            context,
        )?;

        if !field.fields.is_empty()
            && let Some(child) = value.as_object()
        {
            let parent = namespace(parent, name);
            validate_fields(validator, context, errors, &parent, &field.fields, child)?;
        }
    }

    Ok(())
}

fn parse_fields(parent: &str, value: &JsonValue) -> Result<BTreeMap<String, FieldSchema>, Error> {
    let object = value.as_object().ok_or_else(|| {
        invalid_schema(format!("field '{parent}' nested fields must be an object"))
    })?;

    object
        .iter()
        .map(|(name, field)| Ok((name.clone(), FieldSchema::from_value(name, field)?)))
        .collect()
}

fn parse_rules(field: &str, value: &JsonValue) -> Result<Vec<RuleGroup>, Error> {
    match value {
        JsonValue::Array(rules) => rules
            .iter()
            .map(|rule| parse_rule_item(field, rule))
            .collect::<Result<Vec<_>, _>>()
            .map(|groups| groups.into_iter().flatten().collect()),
        JsonValue::String(expr) => parse_rule_expression(expr),
        _ => Err(invalid_schema(format!(
            "field '{field}' rules must be a string or array"
        ))),
    }
}

fn parse_rule_item(field: &str, value: &JsonValue) -> Result<Vec<RuleGroup>, Error> {
    match value {
        JsonValue::String(expr) => parse_rule_expression(expr),
        JsonValue::Object(object) => {
            if object.len() != 1 {
                return Err(invalid_schema(format!(
                    "field '{field}' rule object must contain exactly one rule"
                )));
            }

            let (name, params) = object
                .iter()
                .next()
                .expect("rule object must contain one entry");
            Ok(vec![RuleGroup::Single(parse_rule_object(name, params)?)])
        }
        _ => Err(invalid_schema(format!(
            "field '{field}' rule item must be a string or object"
        ))),
    }
}

fn parse_rule_object(name: &str, params: &JsonValue) -> Result<RuleSpec, Error> {
    if params.is_null() {
        return Ok(RuleSpec::new(name));
    }

    if let Some(object) = params.as_object() {
        let mut spec = RuleSpec::new(name);
        for (key, value) in object {
            spec = spec.param(key, param_value(value)?);
        }
        return Ok(spec);
    }

    Ok(RuleSpec::new(name).param(default_param_name(name), param_value(params)?))
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
        JsonValue::Null | JsonValue::Object(_) => Err(invalid_schema(
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
        "eq_field" | "ne_field" | "gt_field" | "gte_field" | "lt_field" | "lte_field" => "compare",
        _ => "value",
    }
}

fn compare_param(params: &Params) -> Option<&str> {
    params.get("compare").or_else(|| params.get("value"))
}

fn required_object<'a>(
    value: &'a JsonValue,
    field: &'static str,
) -> Result<&'a serde_json::Map<String, JsonValue>, Error> {
    value
        .get(field)
        .and_then(JsonValue::as_object)
        .ok_or_else(|| invalid_schema(format!("schema must contain object '{field}'")))
}

fn namespace(parent: &str, field: &str) -> String {
    if parent.is_empty() {
        field.to_owned()
    } else {
        format!("{parent}.{field}")
    }
}

fn invalid_schema(reason: impl Into<String>) -> Error {
    Error::InvalidSchema {
        reason: reason.into(),
    }
}
