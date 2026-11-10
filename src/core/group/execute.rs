use std::future::Future;
use std::pin::Pin;

use crate::core::{
    Access, Context, Error, Field, FieldError, FieldParts, Items, Params, Runner, Value,
};
use crate::field_error;
use crate::target::FieldTarget;

use super::model::{Check, CheckOutput, Execution, FieldMeta, Flow, Group, Scope, Step};

impl Group {
    pub(crate) async fn execute<V: Value>(
        &self,
        errors: &mut Vec<FieldError>,
        target: FieldTarget<'_>,
        value: &V,
        context: &Context<'_>,
    ) -> Result<(), Error> {
        let target = FieldMeta::new(target);
        self.declared_params::<V>(&target, Some(value), context, Scope::default())?;
        self.run(errors, &target, value, context, None, Scope::default())
            .await
            .map(|_| ())
    }

    pub(crate) async fn execute_spec<V: Value, A: Access>(
        &self,
        errors: &mut Vec<FieldError>,
        target: FieldTarget<'_>,
        value: &V,
        context: &Context<'_>,
        access: &A,
    ) -> Result<Flow, Error> {
        let target = FieldMeta::new(target);
        self.run(
            errors,
            &target,
            value,
            context,
            None,
            Scope {
                access: Some(access),
                items: None,
            },
        )
        .await
    }

    pub(crate) async fn execute_spec_with_items<V: Value, A: Access, I: Items>(
        &self,
        errors: &mut Vec<FieldError>,
        target: FieldTarget<'_>,
        value: &V,
        context: &Context<'_>,
        access: &A,
        items: &I,
    ) -> Result<Flow, Error> {
        let target = FieldMeta::new(target);
        self.run(
            errors,
            &target,
            value,
            context,
            None,
            Scope {
                access: Some(access),
                items: Some(items),
            },
        )
        .await
    }

    pub(crate) async fn execute_with_fields<V: Value, A: Access>(
        &self,
        errors: &mut Vec<FieldError>,
        target: FieldTarget<'_>,
        value: &V,
        context: &Context<'_>,
        access: &A,
    ) -> Result<(), Error> {
        self.execute_spec(errors, target, value, context, access)
            .await
            .map(|_| ())
    }

    pub(crate) async fn execute_with_fields_and_items<V: Value, A: Access, I: Items>(
        &self,
        errors: &mut Vec<FieldError>,
        target: FieldTarget<'_>,
        value: &V,
        context: &Context<'_>,
        access: &A,
        items: &I,
    ) -> Result<(), Error> {
        self.execute_spec_with_items(errors, target, value, context, access, items)
            .await
            .map(|_| ())
    }

    fn run<'a, V: Value + 'a>(
        &'a self,
        errors: &'a mut Vec<FieldError>,
        target: &'a FieldMeta<'a>,
        value: &'a V,
        context: &'a Context<'a>,
        display_rule: Option<&'a str>,
        scope: Scope<'a>,
    ) -> Pin<Box<dyn Future<Output = Result<Flow, Error>> + 'a>> {
        Box::pin(async move {
            let exec = Execution {
                context,
                display_rule,
                scope,
            };

            for step in &self.steps {
                match step {
                    Step::Check(check) => {
                        if self
                            .execute_check(errors, target, value, check, &exec)
                            .await?
                            == Flow::Stop
                        {
                            return Ok(Flow::Stop);
                        }
                    }
                    Step::Any { checks, reason } => {
                        let mut passes = false;
                        for check in checks {
                            match self.evaluate(target, value, check, &exec).await? {
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
                                target.target.clone(),
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
        })
    }

    async fn execute_check<V: Value>(
        &self,
        errors: &mut Vec<FieldError>,
        target: &FieldMeta<'_>,
        value: &V,
        check: &Check,
        exec: &Execution<'_, '_, '_>,
    ) -> Result<Flow, Error> {
        match check {
            Check::OmitEmpty => {
                if value.is_empty_value() {
                    return Ok(Flow::Stop);
                }
            }
            Check::Rule {
                name,
                params,
                handler,
            } => {
                if !self
                    .rule_passes(
                        target,
                        value,
                        exec.context,
                        params,
                        handler.as_ref(),
                        exec.scope,
                    )
                    .await?
                {
                    errors.push(field_error(
                        target.target.clone(),
                        value.kind(),
                        exec.display_rule.unwrap_or(name),
                        name,
                        params.clone(),
                    ));
                }
            }
            Check::Alias { name, group } => {
                return group
                    .run(
                        errors,
                        target,
                        value,
                        exec.context,
                        Some(exec.display_rule.unwrap_or(name)),
                        exec.scope,
                    )
                    .await;
            }
        }

        Ok(Flow::Continue)
    }

    async fn evaluate<V: Value>(
        &self,
        target: &FieldMeta<'_>,
        value: &V,
        check: &Check,
        exec: &Execution<'_, '_, '_>,
    ) -> Result<CheckOutput, Error> {
        match check {
            Check::OmitEmpty => Ok(if value.is_empty_value() {
                CheckOutput::Stop
            } else {
                CheckOutput::Pass
            }),
            Check::Rule {
                name: _,
                params,
                handler,
            } => Ok(
                if self
                    .rule_passes(
                        target,
                        value,
                        exec.context,
                        params,
                        handler.as_ref(),
                        exec.scope,
                    )
                    .await?
                {
                    CheckOutput::Pass
                } else {
                    CheckOutput::Fail
                },
            ),
            Check::Alias { name: _, group } => {
                let mut errors = Vec::new();
                let flow = group
                    .run(&mut errors, target, value, exec.context, None, exec.scope)
                    .await?;
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

    async fn rule_passes<V: Value>(
        &self,
        target: &FieldMeta<'_>,
        value: &V,
        context: &Context<'_>,
        params: &Params,
        handler: &dyn Runner,
        scope: Scope<'_>,
    ) -> Result<bool, Error> {
        if value.is_none() && !handler.validates_none() {
            return Ok(true);
        }

        let field = Field::for_validation(
            FieldParts {
                namespace: &target.namespace,
                struct_namespace: &target.struct_namespace,
                field: target.target.field_name.as_ref(),
                struct_field: target.target.struct_field_name.as_ref(),
                params,
                value,
            },
            context,
            scope.access,
            scope.items,
        );
        handler.check(&field).await
    }

    pub(super) fn with_field<V, T>(
        target: &FieldMeta<'_>,
        value: &V,
        context: &Context<'_>,
        params: &Params,
        scope: Scope<'_>,
        call: impl FnOnce(&Field<'_>) -> Result<T, Error>,
    ) -> Result<T, Error>
    where
        V: Value,
    {
        let field = Field::for_validation(
            FieldParts {
                namespace: &target.namespace,
                struct_namespace: &target.struct_namespace,
                field: target.target.field_name.as_ref(),
                struct_field: target.target.struct_field_name.as_ref(),
                params,
                value,
            },
            context,
            scope.access,
            scope.items,
        );

        call(&field)
    }
}
