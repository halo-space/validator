use crate::core::{Access, Context, Flow, Items, Spec};
use crate::target::{FieldTarget, push_nested_errors};
use crate::traits::{Selective, Validate};
use crate::{Error, FieldError, Kind, Params, Value, field_error, valid};

use super::Validator;

impl Validator {
    #[doc(hidden)]
    pub fn __validate_params<V, A>(
        &self,
        target: FieldTarget<'_>,
        value: &V,
        spec: Spec,
        context: &Context<'_>,
        access: &A,
    ) -> Result<(), Error>
    where
        V: Value,
        A: Access,
    {
        let group = self.compile_spec(spec, false)?;
        group.validate_spec(target, value, context, access)
    }

    #[doc(hidden)]
    pub fn __validate_type_params<V, A>(
        &self,
        target: FieldTarget<'_>,
        spec: Spec,
        context: &Context<'_>,
        access: &A,
    ) -> Result<(), Error>
    where
        V: Value,
        A: Access,
    {
        let group = self.compile_spec(spec, false)?;
        group.validate_type_spec::<V, A>(target, context, access)
    }

    #[doc(hidden)]
    pub fn __validate_spec<V, A>(
        &self,
        errors: &mut Vec<FieldError>,
        target: FieldTarget<'_>,
        value: &V,
        spec: Spec,
        context: &Context<'_>,
        access: &A,
    ) -> Result<bool, Error>
    where
        V: Value,
        A: Access,
    {
        let group = self.compile_spec(spec, false)?;
        group
            .execute_spec(errors, target, value, context, access)
            .map(|flow| flow == Flow::Stop)
    }

    #[doc(hidden)]
    pub fn __validate_item_params<A, I>(
        &self,
        target: FieldTarget<'_>,
        spec: Spec,
        context: &Context<'_>,
        access: &A,
        items: &I,
    ) -> Result<(), Error>
    where
        A: Access,
        I: Items + Value,
    {
        let group = self.compile_spec(spec, true)?;
        group.validate_spec_with_items(target, items, context, access, items)
    }

    #[doc(hidden)]
    pub fn __validate_items<A, I>(
        &self,
        errors: &mut Vec<FieldError>,
        target: FieldTarget<'_>,
        spec: Spec,
        context: &Context<'_>,
        access: &A,
        items: &I,
    ) -> Result<bool, Error>
    where
        A: Access,
        I: Items + Value,
    {
        let group = self.compile_spec(spec, true)?;
        group
            .execute_spec_with_items(errors, target, items, context, access, items)
            .map(|flow| flow == Flow::Stop)
    }

    #[doc(hidden)]
    pub fn __skip_empty<V: Value>(&self, value: &V) -> bool {
        !value.required()
    }

    #[doc(hidden)]
    pub fn __validate_required_option<T>(
        &self,
        errors: &mut Vec<FieldError>,
        target: FieldTarget<'_>,
        value: &Option<T>,
    ) {
        if value.is_none() {
            errors.push(field_error(
                target,
                Kind::Option,
                "required",
                "required",
                Params::new(),
            ));
        }
    }

    #[doc(hidden)]
    pub fn __validate_nested<T: Validate + Selective>(
        &self,
        errors: &mut Vec<FieldError>,
        target: FieldTarget<'_>,
        value: &T,
        context: &Context<'_>,
    ) -> Result<(), Error> {
        let child = context.child(target.struct_field_name.as_ref());
        match value.__validate_with_context(self, &child) {
            Ok(()) => {}
            Err(nested) if nested.is_failed() => push_nested_errors(errors, target, nested),
            Err(error) => return Err(error),
        }
        Ok(())
    }

    #[doc(hidden)]
    pub fn __validate_nested_option<T: Validate + Selective>(
        &self,
        errors: &mut Vec<FieldError>,
        target: FieldTarget<'_>,
        value: &Option<T>,
        context: &Context<'_>,
    ) -> Result<(), Error> {
        if let Some(value) = value {
            self.__validate_nested(errors, target, value, context)?;
        }
        Ok(())
    }

    #[doc(hidden)]
    pub fn __valid<'a>(
        &self,
        type_name: &'a str,
        errors: &'a mut Vec<FieldError>,
        kind: &'a dyn Fn(&str) -> Kind,
    ) -> valid::Valid<'a> {
        valid::Valid::new(type_name, errors, kind)
    }

    #[doc(hidden)]
    pub fn __retain_selected_struct_errors(
        &self,
        errors: &mut Vec<FieldError>,
        start: usize,
        context: &Context<'_>,
    ) {
        if context.is_all() || start == errors.len() {
            return;
        }

        let mut index = 0;
        errors.retain(|error| {
            let selected = index < start || context.includes(error.struct_field());
            index += 1;
            selected
        });
    }
}
