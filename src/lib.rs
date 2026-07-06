mod core;
mod i18n;
mod rules;

use self::core::{Aliases, Rules};
pub use self::core::{
    Args, Error, Errors, Field, FieldError, FloatKind, IntKind, Kind, Namespace, Number, Rule,
    UintKind, Value,
};
pub use validator_derive::Validate;

pub mod prelude {
    pub use crate::{
        Args, Error, Errors, Field, FieldError, FloatKind, IntKind, Kind, Namespace, Number, Rule,
        Validate, Validator,
    };
}

pub struct Validator {
    rules: Rules,
    aliases: Aliases,
}

impl Validator {
    pub fn new() -> Self {
        let mut rules = Rules::new();
        crate::rules::load_rules(&mut rules).expect("default validator rules must be valid");

        let mut aliases = Aliases::new();
        crate::rules::load_aliases(&mut aliases).expect("default validator aliases must be valid");

        Self { rules, aliases }
    }

    pub fn validate<T: Validate>(&self, value: &T) -> Result<(), Errors> {
        value.validate(self)
    }

    pub fn rule<R>(mut self, name: impl Into<String>, rule: R) -> Result<Self, Error>
    where
        R: Rule + Send + Sync + 'static,
    {
        self.rules.insert(name, rule)?;
        Ok(self)
    }

    pub fn alias(mut self, name: impl Into<String>, expr: impl AsRef<str>) -> Result<Self, Error> {
        self.aliases.insert(name, expr)?;
        Ok(self)
    }

    #[doc(hidden)]
    pub fn __validate_rule<V: Value>(
        &self,
        errors: &mut Errors,
        target: FieldTarget<'_>,
        value: &V,
        rule: &str,
        args: Args,
    ) {
        self.__validate_rule_with_display(errors, target, value, rule, rule, args);
    }

    #[doc(hidden)]
    pub fn __validate_alias<V: Value>(
        &self,
        errors: &mut Errors,
        target: FieldTarget<'_>,
        value: &V,
        alias: &str,
    ) {
        let Some(specs) = self.aliases.get(alias) else {
            errors.push(field_error(target, alias, "alias", Args::new()));
            return;
        };

        for group in specs {
            if let Some(spec) = group.single() {
                if spec.name() == "omitempty" {
                    if self.__skip_empty(value) {
                        return;
                    }
                    continue;
                }

                self.__validate_rule_with_display(
                    errors,
                    target,
                    value,
                    alias,
                    spec.name(),
                    spec.args().clone(),
                );
                continue;
            }

            if let Some(alternatives) = group.alternatives() {
                if alternatives
                    .iter()
                    .any(|spec| self.__rule_passes(target, value, spec.name(), spec.args()))
                {
                    continue;
                }

                let actual_rule = alternatives
                    .iter()
                    .map(|spec| spec.name())
                    .collect::<Vec<_>>()
                    .join("|");
                errors.push(field_error(target, alias, &actual_rule, Args::new()));
            }
        }
    }

    #[doc(hidden)]
    pub fn __skip_empty<V: Value>(&self, value: &V) -> bool {
        !value.required()
    }

    fn __validate_rule_with_display<V: Value>(
        &self,
        errors: &mut Errors,
        target: FieldTarget<'_>,
        value: &V,
        rule: &str,
        actual_rule: &str,
        args: Args,
    ) {
        if !self.__rule_passes(target, value, actual_rule, &args) {
            errors.push(field_error(target, rule, actual_rule, args));
        }
    }

    fn __rule_passes<V: Value>(
        &self,
        target: FieldTarget<'_>,
        value: &V,
        actual_rule: &str,
        args: &Args,
    ) -> bool {
        if actual_rule == "omitempty" {
            return true;
        }

        if actual_rule != "required" && value.is_none() {
            return true;
        }

        let Some(handler) = self.rules.get(actual_rule) else {
            return false;
        };

        let namespace = Namespace::new(format!("{}.{}", target.type_name, target.field_name));
        let struct_namespace =
            Namespace::new(format!("{}.{}", target.type_name, target.struct_field_name));
        let field = Field::new(
            &namespace,
            &struct_namespace,
            target.field_name,
            target.struct_field_name,
            args,
            value,
        );

        handler.check(&field)
    }
}

impl Default for Validator {
    fn default() -> Self {
        Self::new()
    }
}

pub trait Validate {
    fn validate(&self, validator: &Validator) -> Result<(), Errors>;
}

#[derive(Clone, Copy)]
#[doc(hidden)]
pub struct FieldTarget<'a> {
    pub type_name: &'a str,
    pub field_name: &'a str,
    pub struct_field_name: &'a str,
}

impl<'a> FieldTarget<'a> {
    pub fn new(type_name: &'a str, field_name: &'a str, struct_field_name: &'a str) -> Self {
        Self {
            type_name,
            field_name,
            struct_field_name,
        }
    }
}

fn field_error(target: FieldTarget<'_>, rule: &str, actual_rule: &str, args: Args) -> FieldError {
    FieldError::new(
        Namespace::new(format!("{}.{}", target.type_name, target.field_name)),
        Namespace::new(format!("{}.{}", target.type_name, target.struct_field_name)),
        target.field_name,
        target.struct_field_name,
        rule,
        actual_rule,
        args,
    )
}
