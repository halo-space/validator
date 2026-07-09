use std::sync::Arc;

use super::{
    Aliases, Context, Error, Expr, Field, FieldError, Fields, Namespace, Params, Rule, Rules, Value,
};
use crate::{FieldTarget, field_error, field_rule_passes, is_field_rule, namespace_for};

struct Exec<'a, 'b> {
    context: &'a Context,
    display_rule: Option<&'a str>,
    fields: Option<&'a Fields<'b>>,
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
    Field {
        name: String,
        params: Params,
    },
    OmitEmpty,
}

enum CompileMode {
    Direct,
    Field,
    Alias,
}

#[derive(Clone, Copy, Eq, PartialEq)]
enum Flow {
    Continue,
    Stop,
}

impl Group {
    pub(crate) fn compile(exprs: &[Expr], rules: &Rules, aliases: &Aliases) -> Result<Self, Error> {
        Self::build(exprs, rules, aliases, CompileMode::Direct)
    }

    pub(crate) fn compile_with_fields(
        exprs: &[Expr],
        rules: &Rules,
        aliases: &Aliases,
    ) -> Result<Self, Error> {
        Self::build(exprs, rules, aliases, CompileMode::Field)
    }

    pub(crate) fn compile_alias(
        exprs: &[Expr],
        rules: &Rules,
        aliases: &Aliases,
    ) -> Result<Self, Error> {
        Self::build(exprs, rules, aliases, CompileMode::Alias)
    }

    pub(crate) fn execute<V: Value>(
        &self,
        errors: &mut Vec<FieldError>,
        target: FieldTarget<'_>,
        value: &V,
        context: &Context,
    ) -> Result<(), Error> {
        self.execute_with_display(errors, target, value, context, None, None)
    }

    pub(crate) fn execute_with_fields<'a, V, F>(
        &self,
        errors: &mut Vec<FieldError>,
        target: FieldTarget<'_>,
        value: &V,
        context: &Context,
        fields: F,
    ) -> Result<(), Error>
    where
        V: Value,
        F: Fn(&str) -> Option<&'a dyn Value> + 'a,
    {
        self.execute_with_display(errors, target, value, context, None, Some(&fields))
    }

    pub(crate) fn execute_alias<V: Value>(
        &self,
        errors: &mut Vec<FieldError>,
        target: FieldTarget<'_>,
        value: &V,
        alias: &str,
        context: &Context,
    ) -> Result<(), Error> {
        self.execute_with_display(errors, target, value, context, Some(alias), None)
    }

    fn build(
        exprs: &[Expr],
        rules: &Rules,
        aliases: &Aliases,
        mode: CompileMode,
    ) -> Result<Self, Error> {
        let mut steps = Vec::new();

        for expr in exprs {
            if let Some(spec) = expr.single() {
                steps.push(Step::Check(Self::build_check(
                    spec.name(),
                    spec.params(),
                    rules,
                    aliases,
                    &mode,
                )?));
                continue;
            }

            if let Some(alternatives) = expr.alternatives() {
                let checks = alternatives
                    .iter()
                    .map(|spec| {
                        Self::build_check(spec.name(), spec.params(), rules, aliases, &mode)
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
        params: &Params,
        rules: &Rules,
        aliases: &Aliases,
        mode: &CompileMode,
    ) -> Result<Check, Error> {
        if name == "omitempty" {
            return Ok(Check::OmitEmpty);
        }

        if matches!(mode, CompileMode::Field) && is_field_rule(name) {
            return Ok(Check::Field {
                name: name.to_owned(),
                params: params.clone(),
            });
        }

        if let Some(handler) = rules.get(name) {
            return Ok(Check::Rule {
                name: name.to_owned(),
                params: params.clone(),
                handler,
            });
        }

        if !matches!(mode, CompileMode::Alias)
            && let Some(exprs) = aliases.get(name)
        {
            let group = Self::build(exprs, rules, aliases, CompileMode::Alias)?;
            return Ok(Check::Alias {
                name: name.to_owned(),
                group: Arc::new(group),
            });
        }

        Err(Error::UnknownRule {
            name: name.to_owned(),
        })
    }

    fn execute_with_display<V: Value>(
        &self,
        errors: &mut Vec<FieldError>,
        target: FieldTarget<'_>,
        value: &V,
        context: &Context,
        display_rule: Option<&str>,
        fields: Option<&Fields<'_>>,
    ) -> Result<(), Error> {
        let exec = Exec {
            context,
            display_rule,
            fields,
        };

        for step in &self.steps {
            match step {
                Step::Check(check) => {
                    if self.execute_check(errors, target.clone(), value, check, &exec)?
                        == Flow::Stop
                    {
                        return Ok(());
                    }
                }
                Step::Any { checks, reason } => {
                    let mut passes = false;
                    for check in checks {
                        if self.check_passes(target.clone(), value, check, &exec)? {
                            passes = true;
                            break;
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

        Ok(())
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
                    name,
                    params,
                    handler.clone(),
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
                group.execute_with_display(
                    errors,
                    target,
                    value,
                    exec.context,
                    Some(name),
                    exec.fields,
                )?;
            }
            Check::Field { name, params } => {
                if !field_rule_passes(value, params, exec.fields, name) {
                    errors.push(field_error(
                        target,
                        value.kind(),
                        name,
                        name,
                        params.clone(),
                    ));
                }
            }
        }

        Ok(Flow::Continue)
    }

    fn check_passes<V: Value>(
        &self,
        target: FieldTarget<'_>,
        value: &V,
        check: &Check,
        exec: &Exec<'_, '_>,
    ) -> Result<bool, Error> {
        match check {
            Check::OmitEmpty => Ok(true),
            Check::Rule {
                name,
                params,
                handler,
            } => self.rule_passes(target, value, exec.context, name, params, handler.clone()),
            Check::Alias { name: _, group } => {
                let mut errors = Vec::new();
                group.execute_with_display(
                    &mut errors,
                    target,
                    value,
                    exec.context,
                    None,
                    exec.fields,
                )?;
                Ok(errors.is_empty())
            }
            Check::Field { name, params } => {
                Ok(field_rule_passes(value, params, exec.fields, name))
            }
        }
    }

    fn rule_passes<V: Value>(
        &self,
        target: FieldTarget<'_>,
        value: &V,
        context: &Context,
        name: &str,
        params: &Params,
        handler: Arc<dyn Rule>,
    ) -> Result<bool, Error> {
        if name != "required" && value.is_none() {
            return Ok(true);
        }

        let namespace = Namespace::new(namespace_for(&target.type_name, &target.field_name));
        let struct_namespace =
            Namespace::new(namespace_for(&target.type_name, &target.struct_field_name));
        let field = Field::with_context(
            &namespace,
            &struct_namespace,
            target.field_name.as_ref(),
            target.struct_field_name.as_ref(),
            params,
            value,
            context,
        );

        handler.check(&field)
    }
}
