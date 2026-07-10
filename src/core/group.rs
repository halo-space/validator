use std::sync::Arc;

use super::{
    Access, Context, Entry, Error, Expr, Field, FieldError, Namespace, Params, Registry, Rule,
    Value,
};
use crate::{FieldTarget, field_error, namespace_for};

struct Exec<'a, 'b> {
    context: &'a Context,
    display_rule: Option<&'a str>,
    access: Option<&'b dyn Access>,
}

#[derive(Clone)]
pub(crate) struct Group {
    steps: Vec<Step>,
}

#[derive(Clone)]
enum Step {
    Check(Check),
    Any { checks: Vec<Check>, reason: String },
}

#[derive(Clone)]
enum Check {
    Rule {
        name: String,
        params: Params,
        handler: Arc<dyn Rule>,
    },
    Alias {
        name: String,
        group: Arc<Group>,
    },
    OmitEmpty,
}

#[derive(Clone, Copy)]
struct CompileMode {
    fields: bool,
}

#[derive(Clone, Copy, Eq, PartialEq)]
pub(crate) enum Flow {
    Continue,
    Stop,
}

enum CheckResult {
    Pass,
    Fail,
    Stop,
}

impl Group {
    pub(crate) fn compile(exprs: &[Expr], registry: &Registry) -> Result<Self, Error> {
        Self::build(
            exprs,
            registry,
            CompileMode { fields: false },
            &mut Vec::new(),
        )
    }

    pub(crate) fn compile_with_fields(exprs: &[Expr], registry: &Registry) -> Result<Self, Error> {
        Self::build(
            exprs,
            registry,
            CompileMode { fields: true },
            &mut Vec::new(),
        )
    }

    pub(crate) fn compile_spec(spec: &super::Spec, registry: &Registry) -> Result<Self, Error> {
        Self::build(
            &[Expr::Single(spec.clone())],
            registry,
            CompileMode { fields: true },
            &mut Vec::new(),
        )
    }

    pub(crate) fn execute<V: Value>(
        &self,
        errors: &mut Vec<FieldError>,
        target: FieldTarget<'_>,
        value: &V,
        context: &Context,
    ) -> Result<(), Error> {
        self.execute_with_display(errors, target, value, context, None, None)
            .map(|_| ())
    }

    pub(crate) fn execute_spec<V, A>(
        &self,
        errors: &mut Vec<FieldError>,
        target: FieldTarget<'_>,
        value: &V,
        context: &Context,
        access: &A,
    ) -> Result<Flow, Error>
    where
        V: Value,
        A: Access,
    {
        self.execute_with_display(errors, target, value, context, None, Some(access))
    }

    pub(crate) fn execute_with_fields<V, A>(
        &self,
        errors: &mut Vec<FieldError>,
        target: FieldTarget<'_>,
        value: &V,
        context: &Context,
        access: &A,
    ) -> Result<(), Error>
    where
        V: Value,
        A: Access,
    {
        self.execute_with_display(errors, target, value, context, None, Some(access))
            .map(|_| ())
    }

    fn build(
        exprs: &[Expr],
        registry: &Registry,
        mode: CompileMode,
        aliases: &mut Vec<String>,
    ) -> Result<Self, Error> {
        let mut steps = Vec::new();

        for expr in exprs {
            if let Some(spec) = expr.single() {
                steps.push(Step::Check(Self::build_check(
                    spec.name(),
                    spec.params(),
                    registry,
                    &mode,
                    aliases,
                )?));
                continue;
            }

            if let Some(alternatives) = expr.alternatives() {
                let checks = alternatives
                    .iter()
                    .map(|spec| {
                        Self::build_check(spec.name(), spec.params(), registry, &mode, aliases)
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
        params: &super::RawParams,
        registry: &Registry,
        mode: &CompileMode,
        aliases: &mut Vec<String>,
    ) -> Result<Check, Error> {
        if name == "omitempty" {
            return Ok(Check::OmitEmpty);
        }

        match registry.get(name) {
            Some(Entry::Rule(handler)) => {
                let signature = handler.signature();
                let params = signature.bind(name, params)?;
                if signature.requires_fields() && !mode.fields {
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
                let group = Self::build(exprs, registry, *mode, aliases)?;
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

    fn execute_with_display<V: Value>(
        &self,
        errors: &mut Vec<FieldError>,
        target: FieldTarget<'_>,
        value: &V,
        context: &Context,
        display_rule: Option<&str>,
        access: Option<&dyn Access>,
    ) -> Result<Flow, Error> {
        let exec = Exec {
            context,
            display_rule,
            access,
        };

        for step in &self.steps {
            match step {
                Step::Check(check) => {
                    if self.execute_check(errors, target.clone(), value, check, &exec)?
                        == Flow::Stop
                    {
                        return Ok(Flow::Stop);
                    }
                }
                Step::Any { checks, reason } => {
                    let mut passes = false;
                    for check in checks {
                        match self.evaluate(target.clone(), value, check, &exec)? {
                            CheckResult::Pass => {
                                passes = true;
                                break;
                            }
                            CheckResult::Fail => {}
                            CheckResult::Stop => return Ok(Flow::Stop),
                        }
                    }

                    if !passes {
                        let rule = exec.display_rule.unwrap_or(reason);
                        errors.push(field_error(
                            target.clone(),
                            value.kind(),
                            rule,
                            reason,
                            Params::new(),
                        ));
                    }
                }
            }
        }

        Ok(Flow::Continue)
    }

    fn execute_check<V: Value>(
        &self,
        errors: &mut Vec<FieldError>,
        target: FieldTarget<'_>,
        value: &V,
        check: &Check,
        exec: &Exec<'_, '_>,
    ) -> Result<Flow, Error> {
        match check {
            Check::OmitEmpty => {
                if !value.required() {
                    return Ok(Flow::Stop);
                }
            }
            Check::Rule {
                name,
                params,
                handler,
            } => {
                if !self.rule_passes(
                    target.clone(),
                    value,
                    exec.context,
                    params,
                    handler.clone(),
                    exec.access,
                )? {
                    errors.push(field_error(
                        target,
                        value.kind(),
                        exec.display_rule.unwrap_or(name),
                        name,
                        params.clone(),
                    ));
                }
            }
            Check::Alias { name, group } => {
                return group.execute_with_display(
                    errors,
                    target,
                    value,
                    exec.context,
                    Some(name),
                    exec.access,
                );
            }
        }

        Ok(Flow::Continue)
    }

    fn evaluate<V: Value>(
        &self,
        target: FieldTarget<'_>,
        value: &V,
        check: &Check,
        exec: &Exec<'_, '_>,
    ) -> Result<CheckResult, Error> {
        match check {
            Check::OmitEmpty => Ok(if value.required() {
                CheckResult::Pass
            } else {
                CheckResult::Stop
            }),
            Check::Rule {
                name: _,
                params,
                handler,
            } => self
                .rule_passes(
                    target,
                    value,
                    exec.context,
                    params,
                    handler.clone(),
                    exec.access,
                )
                .map(|passes| {
                    if passes {
                        CheckResult::Pass
                    } else {
                        CheckResult::Fail
                    }
                }),
            Check::Alias { name: _, group } => {
                let mut errors = Vec::new();
                let flow = group.execute_with_display(
                    &mut errors,
                    target,
                    value,
                    exec.context,
                    None,
                    exec.access,
                )?;
                if flow == Flow::Stop && errors.is_empty() {
                    Ok(CheckResult::Stop)
                } else if errors.is_empty() {
                    Ok(CheckResult::Pass)
                } else {
                    Ok(CheckResult::Fail)
                }
            }
        }
    }

    fn rule_passes<V: Value>(
        &self,
        target: FieldTarget<'_>,
        value: &V,
        context: &Context,
        params: &Params,
        handler: Arc<dyn Rule>,
        access: Option<&dyn Access>,
    ) -> Result<bool, Error> {
        if value.is_none() && !handler.validates_none() {
            return Ok(true);
        }

        let namespace = Namespace::new(namespace_for(&target.type_name, &target.field_name));
        let struct_namespace =
            Namespace::new(namespace_for(&target.type_name, &target.struct_field_name));
        let field = Field::new(
            &namespace,
            &struct_namespace,
            target.field_name.as_ref(),
            target.struct_field_name.as_ref(),
            params,
            value,
        )
        .with_context(context, access);

        handler.check(&field)
    }
}
