use std::sync::Arc;

use super::{
    Aliases, Context, Error, Field, FieldError, Namespace, Params, Rule, RuleGroup, Rules, Value,
};
use crate::{FieldTarget, field_error, namespace_for};

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

enum CompileMode {
    Direct,
    Alias,
}

#[derive(Clone, Copy, Eq, PartialEq)]
enum Flow {
    Continue,
    Stop,
}

impl Group {
    pub(crate) fn compile(
        groups: &[RuleGroup],
        rules: &Rules,
        aliases: &Aliases,
    ) -> Result<Self, Error> {
        Self::build(groups, rules, aliases, CompileMode::Direct)
    }

    pub(crate) fn execute<V: Value>(
        &self,
        errors: &mut Vec<FieldError>,
        target: FieldTarget<'_>,
        value: &V,
        context: &Context,
    ) -> Result<(), Error> {
        self.execute_with_display(errors, target, value, context, None)
    }

    fn build(
        groups: &[RuleGroup],
        rules: &Rules,
        aliases: &Aliases,
        mode: CompileMode,
    ) -> Result<Self, Error> {
        let mut steps = Vec::new();

        for group in groups {
            if let Some(spec) = group.single() {
                steps.push(Step::Check(Self::build_check(
                    spec.name(),
                    spec.params(),
                    rules,
                    aliases,
                    &mode,
                )?));
                continue;
            }

            if let Some(alternatives) = group.alternatives() {
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

        if let Some(handler) = rules.get(name) {
            return Ok(Check::Rule {
                name: name.to_owned(),
                params: params.clone(),
                handler,
            });
        }

        if matches!(mode, CompileMode::Direct)
            && let Some(groups) = aliases.get(name)
        {
            let group = Self::build(groups, rules, aliases, CompileMode::Alias)?;
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
    ) -> Result<(), Error> {
        for step in &self.steps {
            match step {
                Step::Check(check) => {
                    if self.execute_check(
                        errors,
                        target.clone(),
                        value,
                        context,
                        display_rule,
                        check,
                    )? == Flow::Stop
                    {
                        return Ok(());
                    }
                }
                Step::Any { checks, reason } => {
                    let mut passes = false;
                    for check in checks {
                        if self.check_passes(target.clone(), value, context, check)? {
                            passes = true;
                            break;
                        }
                    }

                    if !passes {
                        let rule = display_rule.unwrap_or(reason);
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
        context: &Context,
        display_rule: Option<&str>,
        check: &Check,
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
                    context,
                    name,
                    params,
                    handler.clone(),
                )? {
                    errors.push(field_error(
                        target,
                        value.kind(),
                        display_rule.unwrap_or(name),
                        name,
                        params.clone(),
                    ));
                }
            }
            Check::Alias { name, group } => {
                group.execute_with_display(errors, target, value, context, Some(name))?;
            }
        }

        Ok(Flow::Continue)
    }

    fn check_passes<V: Value>(
        &self,
        target: FieldTarget<'_>,
        value: &V,
        context: &Context,
        check: &Check,
    ) -> Result<bool, Error> {
        match check {
            Check::OmitEmpty => Ok(true),
            Check::Rule {
                name,
                params,
                handler,
            } => self.rule_passes(target, value, context, name, params, handler.clone()),
            Check::Alias { name: _, group } => {
                let mut errors = Vec::new();
                group.execute_with_display(&mut errors, target, value, context, None)?;
                Ok(errors.is_empty())
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
