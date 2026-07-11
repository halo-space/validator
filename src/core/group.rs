use std::sync::Arc;

use super::{
    Access, Context, Entry, Error, Expr, Field, FieldError, Items, Kind, Namespace, Params,
    Registry, Rule, Value,
};
use crate::{FieldTarget, field_error, namespace_for};

struct Exec<'a, 'b, 'c> {
    context: &'a Context<'c>,
    display_rule: Option<&'a str>,
    scope: Scope<'b>,
}

#[derive(Clone, Copy, Default)]
struct Scope<'a> {
    access: Option<&'a dyn Access>,
    items: Option<&'a dyn Items>,
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

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum Mode {
    Value,
    Fields,
    FieldsWithAliases,
    FieldsAndItems,
    FieldsAndItemsWithAliases,
}

impl Mode {
    const fn fields(self) -> bool {
        !matches!(self, Self::Value)
    }

    const fn items(self) -> bool {
        matches!(self, Self::FieldsAndItems | Self::FieldsAndItemsWithAliases)
    }

    const fn alias(self) -> Self {
        match self {
            Self::FieldsWithAliases | Self::FieldsAndItemsWithAliases => self,
            Self::Value | Self::Fields | Self::FieldsAndItems => Self::Value,
        }
    }
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

struct TypeValue {
    kind: Kind,
}

impl Value for TypeValue {
    fn kind(&self) -> Kind {
        self.kind
    }

    fn required(&self) -> bool {
        false
    }
}

impl Group {
    pub(crate) fn compile(exprs: &[Expr], registry: &Registry) -> Result<Self, Error> {
        Self::build(exprs, registry, Mode::Value, &mut Vec::new())
    }

    pub(crate) fn compile_with_fields(exprs: &[Expr], registry: &Registry) -> Result<Self, Error> {
        Self::build(exprs, registry, Mode::FieldsWithAliases, &mut Vec::new())
    }

    pub(crate) fn compile_spec(spec: &super::Spec, registry: &Registry) -> Result<Self, Error> {
        Self::build(
            &[Expr::Single(spec.clone())],
            registry,
            Mode::Fields,
            &mut Vec::new(),
        )
    }

    pub(crate) fn compile_spec_with_items(
        spec: &super::Spec,
        registry: &Registry,
    ) -> Result<Self, Error> {
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

    pub(crate) fn execute<V: Value>(
        &self,
        errors: &mut Vec<FieldError>,
        target: FieldTarget<'_>,
        value: &V,
        context: &Context<'_>,
    ) -> Result<(), Error> {
        let scope = Scope::default();
        self.validate_declared_params::<V>(target.clone(), Some(value), context, scope)?;
        self.run(errors, target, value, context, None, scope)
            .map(|_| ())
    }

    pub(crate) fn validate_spec<V, A>(
        &self,
        target: FieldTarget<'_>,
        value: &V,
        context: &Context<'_>,
        access: &A,
    ) -> Result<(), Error>
    where
        V: Value,
        A: Access,
    {
        self.validate_declared_params::<V>(
            target,
            Some(value),
            context,
            Scope {
                access: Some(access),
                items: None,
            },
        )
    }

    pub(crate) fn validate_type_spec<V, A>(
        &self,
        target: FieldTarget<'_>,
        context: &Context<'_>,
        access: &A,
    ) -> Result<(), Error>
    where
        V: Value,
        A: Access,
    {
        self.validate_declared_params::<V>(
            target,
            None,
            context,
            Scope {
                access: Some(access),
                items: None,
            },
        )
    }

    pub(crate) fn run_spec<V, A>(
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

    pub(crate) fn validate_spec_with_items<V, A, I>(
        &self,
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
        self.validate_params(
            target,
            value,
            context,
            Scope {
                access: Some(access),
                items: Some(items),
            },
        )
    }

    pub(crate) fn run_spec_with_items<V, A, I>(
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

    pub(crate) fn validate_with_fields<V, A>(
        &self,
        target: FieldTarget<'_>,
        value: &V,
        context: &Context<'_>,
        access: &A,
    ) -> Result<(), Error>
    where
        V: Value,
        A: Access,
    {
        self.validate_params(
            target,
            value,
            context,
            Scope {
                access: Some(access),
                items: None,
            },
        )
    }

    pub(crate) fn run_with_fields<V, A>(
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

    pub(crate) fn run_with_fields_and_items<V, A, I>(
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
        params: &super::RawParams,
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

    fn validate_params<V: Value>(
        &self,
        target: FieldTarget<'_>,
        value: &V,
        context: &Context<'_>,
        scope: Scope<'_>,
    ) -> Result<(), Error> {
        for step in &self.steps {
            match step {
                Step::Check(check) => {
                    self.validate_check(target.clone(), value, context, check, scope)?;
                }
                Step::Any { checks, .. } => {
                    for check in checks {
                        self.validate_check(target.clone(), value, context, check, scope)?;
                    }
                }
            }
        }

        Ok(())
    }

    fn validate_declared_params<V: Value>(
        &self,
        target: FieldTarget<'_>,
        value: Option<&V>,
        context: &Context<'_>,
        scope: Scope<'_>,
    ) -> Result<(), Error> {
        if let Some(kind) = V::declared_kind() {
            return self.validate_params(target, &TypeValue { kind }, context, scope);
        }

        match value {
            Some(value) => self.validate_params(target, value, context, scope),
            None => self.validate_params(target, &TypeValue { kind: Kind::Other }, context, scope),
        }
    }

    fn validate_check<V: Value>(
        &self,
        target: FieldTarget<'_>,
        value: &V,
        context: &Context<'_>,
        check: &Check,
        scope: Scope<'_>,
    ) -> Result<(), Error> {
        match check {
            Check::Rule {
                params, handler, ..
            } => Self::with_field(target, value, context, params, scope, |field| {
                handler.validate_params(field)
            }),
            Check::Alias { group, .. } => group.validate_params(target, value, context, scope),
            Check::OmitEmpty => Ok(()),
        }
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
        let exec = Exec {
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
        exec: &Exec<'_, '_, '_>,
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
        exec: &Exec<'_, '_, '_>,
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
                    exec.scope,
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
                let flow = group.run(&mut errors, target, value, exec.context, None, exec.scope)?;
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

    fn with_field<V, T>(
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

#[cfg(test)]
mod tests {
    use super::Mode;

    #[test]
    fn modes_expose_only_their_declared_context() {
        assert!(!Mode::Value.fields());
        assert!(!Mode::Value.items());

        for mode in [Mode::Fields, Mode::FieldsWithAliases] {
            assert!(mode.fields());
            assert!(!mode.items());
        }

        for mode in [Mode::FieldsAndItems, Mode::FieldsAndItemsWithAliases] {
            assert!(mode.fields());
            assert!(mode.items());
        }
    }

    #[test]
    fn only_alias_modes_preserve_context_inside_aliases() {
        assert_eq!(Mode::Value.alias(), Mode::Value);
        assert_eq!(Mode::Fields.alias(), Mode::Value);
        assert_eq!(Mode::FieldsAndItems.alias(), Mode::Value);
        assert_eq!(Mode::FieldsWithAliases.alias(), Mode::FieldsWithAliases);
        assert_eq!(
            Mode::FieldsAndItemsWithAliases.alias(),
            Mode::FieldsAndItemsWithAliases
        );
    }
}
