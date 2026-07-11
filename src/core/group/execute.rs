use std::sync::Arc;

use crate::core::{
    Access, Context, Error, Field, FieldError, Items, Namespace, Params, Rule, Value,
};
use crate::{FieldTarget, field_error, namespace_for};

use super::model::{Check, CheckOutput, Execution, Flow, Group, Scope, Step};

impl Group {
    pub(crate) fn execute<V: Value>(
        &self,
        errors: &mut Vec<FieldError>,
        target: FieldTarget<'_>,
        value: &V,
        context: &Context<'_>,
    ) -> Result<(), Error> {
        let scope = Scope::default();
        self.declared_params::<V>(target.clone(), Some(value), context, scope)?;
        self.run(errors, target, value, context, None, scope)
            .map(|_| ())
    }

    pub(crate) fn execute_spec<V, A>(
        &self,
        errors: &mut Vec<FieldError>,
        target: FieldTarget<'_>,
        value: &V,
        context: &Context<'_>,
        access: &A,
    ) -> Result<Flow, Error>
    where
        V: Value,
        A: Access,
    {
        self.run(
            errors,
            target,
            value,
            context,
            None,
            Scope {
                access: Some(access),
                items: None,
            },
        )
    }

    pub(crate) fn execute_spec_with_items<V, A, I>(
        &self,
        errors: &mut Vec<FieldError>,
        target: FieldTarget<'_>,
        value: &V,
        context: &Context<'_>,
        access: &A,
        items: &I,
    ) -> Result<Flow, Error>
    where
        V: Value,
        A: Access,
        I: Items,
    {
        self.run(
            errors,
            target,
            value,
            context,
            None,
            Scope {
                access: Some(access),
                items: Some(items),
            },
        )
    }

    pub(crate) fn execute_with_fields<V, A>(
        &self,
        errors: &mut Vec<FieldError>,
        target: FieldTarget<'_>,
        value: &V,
        context: &Context<'_>,
        access: &A,
    ) -> Result<(), Error>
    where
        V: Value,
        A: Access,
    {
        self.run(
            errors,
            target,
            value,
            context,
            None,
            Scope {
                access: Some(access),
                items: None,
            },
        )
        .map(|_| ())
    }

    pub(crate) fn execute_with_fields_and_items<V, A, I>(
        &self,
        errors: &mut Vec<FieldError>,
        target: FieldTarget<'_>,
        value: &V,
        context: &Context<'_>,
        access: &A,
        items: &I,
    ) -> Result<(), Error>
    where
        V: Value,
        A: Access,
        I: Items,
    {
        self.run(
            errors,
            target,
            value,
            context,
            None,
            Scope {
                access: Some(access),
                items: Some(items),
            },
        )
        .map(|_| ())
    }

    fn run<V: Value>(
        &self,
        errors: &mut Vec<FieldError>,
        target: FieldTarget<'_>,
        value: &V,
        context: &Context<'_>,
        display_rule: Option<&str>,
        scope: Scope<'_>,
    ) -> Result<Flow, Error> {
        let exec = Execution {
            context,
            display_rule,
            scope,
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
                            CheckOutput::Pass => {
                                passes = true;
                                break;
                            }
                            CheckOutput::Fail => {}
                            CheckOutput::Stop => return Ok(Flow::Stop),
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
        exec: &Execution<'_, '_, '_>,
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
                    exec.scope,
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
                return group.run(
                    errors,
                    target,
                    value,
                    exec.context,
                    Some(exec.display_rule.unwrap_or(name)),
                    exec.scope,
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
        exec: &Execution<'_, '_, '_>,
    ) -> Result<CheckOutput, Error> {
        match check {
            Check::OmitEmpty => Ok(if value.required() {
                CheckOutput::Pass
            } else {
                CheckOutput::Stop
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
                    exec.scope,
                )
                .map(|passes| {
                    if passes {
                        CheckOutput::Pass
                    } else {
                        CheckOutput::Fail
                    }
                }),
            Check::Alias { name: _, group } => {
                let mut errors = Vec::new();
                let flow = group.run(&mut errors, target, value, exec.context, None, exec.scope)?;
                if flow == Flow::Stop && errors.is_empty() {
                    Ok(CheckOutput::Stop)
                } else if errors.is_empty() {
                    Ok(CheckOutput::Pass)
                } else {
                    Ok(CheckOutput::Fail)
                }
            }
        }
    }

    fn rule_passes<V: Value>(
        &self,
        target: FieldTarget<'_>,
        value: &V,
        context: &Context<'_>,
        params: &Params,
        handler: Arc<dyn Rule>,
        scope: Scope<'_>,
    ) -> Result<bool, Error> {
        if value.is_none() && !handler.validates_none() {
            return Ok(true);
        }

        Self::with_field(target, value, context, params, scope, |field| {
            handler.check(field)
        })
    }

    pub(super) fn with_field<V, T>(
        target: FieldTarget<'_>,
        value: &V,
        context: &Context<'_>,
        params: &Params,
        scope: Scope<'_>,
        call: impl FnOnce(&Field<'_>) -> Result<T, Error>,
    ) -> Result<T, Error>
    where
        V: Value,
    {
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
        .with_context(context, scope.access, scope.items);

        call(&field)
    }
}
