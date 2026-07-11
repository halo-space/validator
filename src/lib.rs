extern crate self as validator;

mod core;
pub mod i18n;
mod rules;
mod schema;
mod target;
pub mod valid;

use std::sync::{Arc, RwLock};

use self::core::{
    Access, CAPACITY, Cache, Context, Expr, Fields, Flow, Group, Items, RawParams, Registry, Spec,
    parse_expression,
};
pub use self::core::{
    Error, Field, FieldError, FloatKind, IntKind, Kind, Namespace, Number, Param, Params, Rule,
    Signature, UintKind, Value,
};
pub use self::schema::Schema;
use self::schema::Tree;
pub use self::target::FieldTarget;
use self::target::push_nested_errors;
pub(crate) use self::target::{field_error, namespace_for};
pub use validator_derive::Validate;

#[doc(hidden)]
pub mod __private {
    pub use crate::Selective;
    pub use crate::core::{
        Access, Context, FieldRef, Projection, RawParams, Resolve, Segment, Spec,
    };
}

pub mod prelude {
    pub use crate::{
        Error, Field, FieldError, FloatKind, IntKind, Kind, Namespace, Number, Param, Params, Rule,
        Schema, Signature, UintKind, Validate, Validator, Value,
    };
}

#[derive(Clone, Eq, Ord, PartialEq, PartialOrd)]
struct SpecKey {
    generation: u64,
    name: String,
    params: RawParams,
    items: bool,
}

pub struct Validator {
    registry: Registry,
    schema: Option<Schema>,
    generation: u64,
    expression_cache: RwLock<Cache<String, Arc<Vec<Expr>>>>,
    compiled_cache: RwLock<Cache<(u64, String), Arc<Group>>>,
    spec_cache: RwLock<Cache<SpecKey, Arc<Group>>>,
    schema_cache: RwLock<Cache<(schema::SchemaId, u64), Arc<Tree>>>,
}

impl Validator {
    pub fn new() -> Self {
        let mut registry = Registry::new();
        crate::rules::load(&mut registry).expect("default validator rules must be valid");
        crate::rules::load_aliases(&mut registry).expect("default validator aliases must be valid");

        Self {
            registry,
            schema: None,
            generation: 0,
            expression_cache: RwLock::new(Cache::new(CAPACITY)),
            compiled_cache: RwLock::new(Cache::new(CAPACITY)),
            spec_cache: RwLock::new(Cache::new(CAPACITY)),
            schema_cache: RwLock::new(Cache::new(CAPACITY)),
        }
    }

    pub fn with_schema(schema: Schema) -> Self {
        let mut validator = Self::new();
        validator.schema = Some(schema);
        validator
    }

    pub fn validate<T: Validate>(&self, value: &T) -> Result<(), Error> {
        value.validate(self)
    }

    /// Validates only the selected relative Rust field paths.
    pub fn partial<T, I, S>(&self, value: &T, fields: I) -> Result<(), Error>
    where
        T: Validate + Selective,
        I: IntoIterator<Item = S>,
        S: AsRef<str>,
    {
        let fields = Fields::new(fields);
        let context = Context::partial(&fields);
        let result = value.__validate_with_context(self, &context);
        fields.verify()?;
        result
    }

    /// Validates every field except the selected relative Rust field paths.
    pub fn except<T, I, S>(&self, value: &T, fields: I) -> Result<(), Error>
    where
        T: Validate + Selective,
        I: IntoIterator<Item = S>,
        S: AsRef<str>,
    {
        let fields = Fields::new(fields);
        let context = Context::except(&fields);
        let result = value.__validate_with_context(self, &context);
        fields.verify()?;
        result
    }

    /// Validates fields for which the relative Namespace predicate returns true.
    pub fn filter<T, F>(&self, value: &T, filter: F) -> Result<(), Error>
    where
        T: Validate + Selective,
        F: Fn(&Namespace) -> bool,
    {
        let context = Context::filter(&filter);
        value.__validate_with_context(self, &context)
    }

    pub fn rule<R>(mut self, name: impl Into<String>, rule: R) -> Result<Self, Error>
    where
        R: Rule + Send + Sync + 'static,
    {
        self.registry.rule(name, rule)?;
        self.bump_generation();
        Ok(self)
    }

    pub fn alias(mut self, name: impl Into<String>, expr: impl AsRef<str>) -> Result<Self, Error> {
        self.registry.alias(name, expr)?;
        self.bump_generation();
        Ok(self)
    }

    pub fn value<V: Value>(&self, value: &V, rules: impl AsRef<str>) -> Result<(), Error> {
        let group = self.compile(rules.as_ref())?;
        let mut errors = Vec::new();
        let target = FieldTarget::value();
        let context = Context::new();
        group.execute(&mut errors, target, value, &context)?;

        if errors.is_empty() {
            Ok(())
        } else {
            Err(Error::failed(errors))
        }
    }

    pub fn validate_map(&self, value: &serde_json::Value) -> Result<(), Error> {
        let schema = self.schema.as_ref().ok_or(Error::MissingSchema)?;
        let tree = self.schema_tree(schema)?;

        self.validate_data(&tree, value)
    }

    pub fn validate_serde<T>(&self, value: &T) -> Result<(), Error>
    where
        T: serde::Serialize + ?Sized,
    {
        let schema = self.schema.as_ref().ok_or(Error::MissingSchema)?;
        let tree = self.schema_tree(schema)?;
        let data = serde_json::to_value(value).map_err(|error| Error::InvalidData {
            reason: error.to_string(),
        })?;

        self.validate_data(&tree, &data)
    }

    fn validate_data(&self, tree: &Tree, value: &serde_json::Value) -> Result<(), Error> {
        let mut errors = Vec::new();
        let context = Context::new();
        tree.validate(&context, &mut errors, value)?;

        if errors.is_empty() {
            Ok(())
        } else {
            Err(Error::failed(errors))
        }
    }

    fn bump_generation(&mut self) {
        self.generation = self.generation.wrapping_add(1);
    }

    fn parse(&self, expression: &str) -> Result<Arc<Vec<Expr>>, Error> {
        if let Some(exprs) = self
            .expression_cache
            .read()
            .expect("expression cache lock must not be poisoned")
            .get(expression)
            .cloned()
        {
            return Ok(exprs);
        }

        let exprs = Arc::new(parse_expression(expression)?);
        let mut cache = self
            .expression_cache
            .write()
            .expect("expression cache lock must not be poisoned");
        if let Some(exprs) = cache.get(expression).cloned() {
            return Ok(exprs);
        }

        cache.insert(expression.to_owned(), exprs.clone());
        Ok(exprs)
    }

    fn compile(&self, expression: &str) -> Result<Arc<Group>, Error> {
        let key = (self.generation, expression.to_owned());
        if let Some(group) = self
            .compiled_cache
            .read()
            .expect("compiled cache lock must not be poisoned")
            .get(&key)
            .cloned()
        {
            return Ok(group);
        }

        let exprs = self.parse(expression)?;
        let group = Arc::new(Group::compile(exprs.as_ref(), &self.registry)?);
        let mut cache = self
            .compiled_cache
            .write()
            .expect("compiled cache lock must not be poisoned");
        if let Some(group) = cache.get(&key).cloned() {
            return Ok(group);
        }

        cache.insert(key, group.clone());
        Ok(group)
    }

    fn schema_tree(&self, schema: &Schema) -> Result<Arc<Tree>, Error> {
        let key = (schema.id(), self.generation);
        if let Some(tree) = self
            .schema_cache
            .read()
            .expect("schema cache lock must not be poisoned")
            .get(&key)
            .cloned()
        {
            return Ok(tree);
        }

        let tree = Arc::new(schema.compile(&self.registry)?);
        let mut cache = self
            .schema_cache
            .write()
            .expect("schema cache lock must not be poisoned");
        if let Some(tree) = cache.get(&key).cloned() {
            return Ok(tree);
        }

        cache.insert(key, tree.clone());
        Ok(tree)
    }

    fn compile_spec(&self, spec: Spec, items: bool) -> Result<Arc<Group>, Error> {
        let key = SpecKey {
            generation: self.generation,
            name: spec.name().to_owned(),
            params: spec.params().clone(),
            items,
        };
        if let Some(group) = self
            .spec_cache
            .read()
            .expect("spec cache lock must not be poisoned")
            .get(&key)
            .cloned()
        {
            return Ok(group);
        }

        let group = Arc::new(if items {
            Group::compile_spec_with_items(&spec, &self.registry)?
        } else {
            Group::compile_spec(&spec, &self.registry)?
        });
        let mut cache = self
            .spec_cache
            .write()
            .expect("spec cache lock must not be poisoned");
        if let Some(group) = cache.get(&key).cloned() {
            return Ok(group);
        }

        cache.insert(key, group.clone());
        Ok(group)
    }

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
            .run_spec(errors, target, value, context, access)
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
            .run_spec_with_items(errors, target, items, context, access, items)
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

impl Default for Validator {
    fn default() -> Self {
        Self::new()
    }
}

pub trait Validate {
    fn validate(&self, validator: &Validator) -> Result<(), Error>;
}

#[doc(hidden)]
pub trait Selective {
    #[doc(hidden)]
    fn __validate_with_context(
        &self,
        validator: &Validator,
        context: &__private::Context<'_>,
    ) -> Result<(), Error>;
}
