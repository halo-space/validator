use std::collections::BTreeMap;

use serde_json::Value as JsonValue;

use crate::core::{Context, Entry, Expr, Group, Registry, Spec};
use crate::target::FieldTarget;
use crate::{Error, FieldError, Params, Value, field_error};

use super::path::parse_path;
use super::validate::{preflight_fields, validate_fields};
use super::value::Projected;
use super::{FieldDef, Fields, TYPE_FAILURE, Type, invalid};

pub(crate) struct Tree {
    root: Scope,
}

impl Tree {
    pub(super) fn compile(
        fields: &BTreeMap<String, FieldDef>,
        registry: &Registry,
    ) -> Result<Self, Error> {
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
                TYPE_FAILURE,
                TYPE_FAILURE,
                params,
            ));
            return Ok(());
        };

        validate_fields(context, errors, "", &self.root, object)
    }
}

pub(super) struct Scope {
    pub(super) fields: BTreeMap<String, Node>,
    pub(super) paths: BTreeMap<String, Path>,
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

    pub(super) fn projected(
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

pub(super) struct Node {
    pub(super) ty: Option<Type>,
    pub(super) group: Group,
    pub(super) children: Fields<Scope>,
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

pub(super) struct Path {
    pub(super) name: String,
    pub(super) segments: Vec<String>,
    pub(super) ty: Option<Type>,
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
