use serde_json::Value as JsonValue;

use crate::core::Context;
use crate::target::FieldTarget;
use crate::{Error, FieldError, Params, Value, field_error};

use super::access::Object;
use super::compile::Scope;
use super::items::Collection;
use super::path::namespace;
use super::{Fields, TYPE_FAILURE, Type};

pub(super) fn validate_fields<'a>(
    context: &Context<'_>,
    errors: &mut Vec<FieldError>,
    parent: &str,
    scope: &'a Scope,
    object: &'a serde_json::Map<String, JsonValue>,
) -> Result<(), Error> {
    let access = Object::new(scope, object);
    for (name, field) in &scope.fields {
        let target = FieldTarget::schema_field(parent, name);
        let value = access
            .get(name)
            .expect("Schema access contains every declared field");

        if value.is_none() {
            field
                .group
                .execute_with_fields(errors, target, value, context, &access)?;
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
                TYPE_FAILURE,
                TYPE_FAILURE,
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
                    field.group.execute_with_fields(
                        errors,
                        target.clone(),
                        value,
                        context,
                        &access,
                    )?;
                }
                Fields::Declared(children) => {
                    let items = Collection::new(values, children);
                    field.group.execute_with_fields_and_items(
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
                .execute_with_fields(errors, target.clone(), value, context, &access)?;

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

pub(super) fn preflight_fields(
    context: &Context<'_>,
    parent: &str,
    scope: &Scope,
) -> Result<(), crate::Error> {
    let object = serde_json::Map::new();
    let access = Object::new(scope, &object);

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
                TYPE_FAILURE,
                TYPE_FAILURE,
                params,
            ));
            continue;
        };

        validate_fields(context, errors, &format!("{array}[{index}]"), scope, object)?;
    }

    Ok(())
}
