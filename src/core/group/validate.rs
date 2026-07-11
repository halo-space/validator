use crate::FieldTarget;
use crate::core::{Access, Context, Error, Items, Kind, Value};

use super::model::{Check, Group, Scope, Step, TypeValue};

impl Group {
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
        self.declared_params::<V>(
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
        self.declared_params::<V>(
            target,
            None,
            context,
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
        self.params(
            target,
            value,
            context,
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
        self.params(
            target,
            value,
            context,
            Scope {
                access: Some(access),
                items: None,
            },
        )
    }

    fn params<V: Value>(
        &self,
        target: FieldTarget<'_>,
        value: &V,
        context: &Context<'_>,
        scope: Scope<'_>,
    ) -> Result<(), Error> {
        for step in &self.steps {
            match step {
                Step::Check(check) => self.check(target.clone(), value, context, check, scope)?,
                Step::Any { checks, .. } => {
                    for check in checks {
                        self.check(target.clone(), value, context, check, scope)?;
                    }
                }
            }
        }
        Ok(())
    }

    pub(super) fn declared_params<V: Value>(
        &self,
        target: FieldTarget<'_>,
        value: Option<&V>,
        context: &Context<'_>,
        scope: Scope<'_>,
    ) -> Result<(), Error> {
        if let Some(kind) = V::declared_kind() {
            return self.params(target, &TypeValue { kind }, context, scope);
        }

        match value {
            Some(value) => self.params(target, value, context, scope),
            None => self.params(target, &TypeValue { kind: Kind::Other }, context, scope),
        }
    }

    fn check<V: Value>(
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
            Check::Alias { group, .. } => group.params(target, value, context, scope),
            Check::OmitEmpty => Ok(()),
        }
    }
}
