use std::sync::Arc;

use super::model::{Check, Group, Mode, Step};
use crate::core::{Entry, Error, Expr, RawParams, Registry, Spec};

impl Group {
    pub(crate) fn compile(exprs: &[Expr], registry: &Registry) -> Result<Self, Error> {
        Self::build(exprs, registry, Mode::Value, &mut Vec::new())
    }

    pub(crate) fn compile_with_fields(exprs: &[Expr], registry: &Registry) -> Result<Self, Error> {
        Self::build(exprs, registry, Mode::FieldsWithAliases, &mut Vec::new())
    }

    pub(crate) fn compile_spec(spec: &Spec, registry: &Registry) -> Result<Self, Error> {
        Self::build(
            &[Expr::Single(spec.clone())],
            registry,
            Mode::Fields,
            &mut Vec::new(),
        )
    }

    pub(crate) fn compile_spec_with_items(spec: &Spec, registry: &Registry) -> Result<Self, Error> {
        Self::build(
            &[Expr::Single(spec.clone())],
            registry,
            Mode::FieldsAndItems,
            &mut Vec::new(),
        )
    }

    pub(crate) fn compile_with_fields_and_items(
        exprs: &[Expr],
        registry: &Registry,
    ) -> Result<Self, Error> {
        Self::build(
            exprs,
            registry,
            Mode::FieldsAndItemsWithAliases,
            &mut Vec::new(),
        )
    }

    fn build(
        exprs: &[Expr],
        registry: &Registry,
        mode: Mode,
        aliases: &mut Vec<String>,
    ) -> Result<Self, Error> {
        let mut steps = Vec::new();

        for expr in exprs {
            if let Some(spec) = expr.single() {
                steps.push(Step::Check(Self::build_check(
                    spec.name(),
                    spec.params(),
                    registry,
                    mode,
                    aliases,
                )?));
                continue;
            }

            if let Some(alternatives) = expr.alternatives() {
                let checks = alternatives
                    .iter()
                    .map(|spec| {
                        Self::build_check(spec.name(), spec.params(), registry, mode, aliases)
                    })
                    .collect::<Result<Vec<_>, _>>()?;
                let reason = alternatives
                    .iter()
                    .map(|spec| spec.name())
                    .collect::<Vec<_>>()
                    .join("|");
                steps.push(Step::Any { checks, reason });
            }
        }

        Ok(Self { steps })
    }

    fn build_check(
        name: &str,
        params: &RawParams,
        registry: &Registry,
        mode: Mode,
        aliases: &mut Vec<String>,
    ) -> Result<Check, Error> {
        if name == "omitempty" {
            return Ok(Check::OmitEmpty);
        }

        match registry.get(name) {
            Some(Entry::Rule(handler)) => {
                let signature = handler.signature();
                let params = signature.bind(name, params)?;
                if signature.requires_fields() && !mode.fields() {
                    return Err(Error::MissingFieldContext {
                        name: name.to_owned(),
                    });
                }
                if signature.requires_items(&params) && !mode.items() {
                    return Err(Error::MissingFieldContext {
                        name: name.to_owned(),
                    });
                }
                Ok(Check::Rule {
                    name: name.to_owned(),
                    params,
                    handler: handler.clone(),
                })
            }
            Some(Entry::Alias(exprs)) => {
                if !params.is_empty() {
                    return Err(Error::InvalidRuleExpression {
                        expression: name.to_owned(),
                        reason: "alias does not accept parameters".to_owned(),
                    });
                }
                if aliases.iter().any(|alias| alias == name) {
                    return Err(Error::RecursiveAlias {
                        name: name.to_owned(),
                    });
                }
                aliases.push(name.to_owned());
                let group = Self::build(exprs, registry, mode.alias(), aliases)?;
                aliases.pop();
                Ok(Check::Alias {
                    name: name.to_owned(),
                    group: Arc::new(group),
                })
            }
            None => Err(Error::UnknownRule {
                name: name.to_owned(),
            }),
        }
    }
}
