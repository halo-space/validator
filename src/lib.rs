extern crate self as validator;

mod core;
pub mod i18n;
mod rules;
mod schema;
pub mod valid;

use std::borrow::Cow;
use std::fmt;
use std::sync::{Arc, RwLock};

use self::core::{
    Access, CAPACITY, Cache, Context, Expr, FieldErrorParts, Flow, Group, Items, RawParams,
    Registry, Spec, parse_expression,
};
pub use self::core::{
    Error, Field, FieldError, FloatKind, IntKind, Kind, Namespace, Number, Param, Params, Rule,
    Signature, UintKind, Value,
};
pub use self::schema::Schema;
use self::schema::Tree;
pub use validator_derive::Validate;

#[doc(hidden)]
pub mod __private {
    pub use crate::core::{Access, Context, FieldRef, Projection, RawParams, Spec};
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
        let context = Context::new();
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
        context: &Context,
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
        context: &Context,
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
        context: &Context,
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
        context: &Context,
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
        context: &Context,
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
    pub fn __validate_nested<T: Validate>(
        &self,
        errors: &mut Vec<FieldError>,
        target: FieldTarget<'_>,
        value: &T,
        context: &Context,
    ) -> Result<(), Error> {
        match value.__validate_with_context(self, context) {
            Ok(()) => {}
            Err(nested) if nested.is_failed() => push_nested_errors(errors, target, nested),
            Err(error) => return Err(error),
        }
        Ok(())
    }

    #[doc(hidden)]
    pub fn __validate_nested_option<T: Validate>(
        &self,
        errors: &mut Vec<FieldError>,
        target: FieldTarget<'_>,
        value: &Option<T>,
        context: &Context,
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
    ) -> valid::Valid<'a> {
        valid::Valid::new(type_name, errors)
    }
}

impl Default for Validator {
    fn default() -> Self {
        Self::new()
    }
}

pub trait Validate {
    fn validate(&self, validator: &Validator) -> Result<(), Error>;

    #[doc(hidden)]
    fn __validate_with_context(
        &self,
        validator: &Validator,
        _context: &__private::Context,
    ) -> Result<(), Error> {
        self.validate(validator)
    }
}

#[derive(Clone)]
#[doc(hidden)]
pub struct FieldTarget<'a> {
    pub type_name: Cow<'a, str>,
    pub field_name: Cow<'a, str>,
    pub struct_field_name: Cow<'a, str>,
}

impl<'a> FieldTarget<'a> {
    pub fn new(type_name: &'a str, field_name: &'a str, struct_field_name: &'a str) -> Self {
        Self {
            type_name: Cow::Borrowed(type_name),
            field_name: Cow::Borrowed(field_name),
            struct_field_name: Cow::Borrowed(struct_field_name),
        }
    }

    pub fn index(&self, index: usize) -> Self {
        Self {
            type_name: self.type_name.clone(),
            field_name: Cow::Owned(format!("{}[{index}]", self.field_name)),
            struct_field_name: Cow::Owned(format!("{}[{index}]", self.struct_field_name)),
        }
    }

    pub fn key<K: fmt::Display>(&self, key: K) -> Self {
        let key = serde_json::to_string(&key.to_string())
            .expect("serializing a map key string must not fail");
        Self {
            type_name: self.type_name.clone(),
            field_name: Cow::Owned(format!("{}[{key}]", self.field_name)),
            struct_field_name: Cow::Owned(format!("{}[{key}]", self.struct_field_name)),
        }
    }

    pub fn value() -> Self {
        Self {
            type_name: Cow::Borrowed(""),
            field_name: Cow::Borrowed("$value"),
            struct_field_name: Cow::Borrowed("$value"),
        }
    }

    pub fn schema(field_name: impl Into<String>) -> Self {
        let field_name = field_name.into();
        Self {
            type_name: Cow::Borrowed(""),
            struct_field_name: Cow::Owned(field_name.clone()),
            field_name: Cow::Owned(field_name),
        }
    }

    pub fn schema_field(parent: &str, field_name: &str) -> Self {
        Self {
            type_name: Cow::Owned(parent.to_owned()),
            field_name: Cow::Owned(field_name.to_owned()),
            struct_field_name: Cow::Owned(field_name.to_owned()),
        }
    }
}

pub(crate) fn field_error(
    target: FieldTarget<'_>,
    kind: Kind,
    rule: &str,
    reason: &str,
    params: Params,
) -> FieldError {
    let namespace = namespace_for(&target.type_name, &target.field_name);
    let struct_namespace = namespace_for(&target.type_name, &target.struct_field_name);

    FieldError::new(FieldErrorParts {
        namespace: Namespace::new(namespace),
        struct_namespace: Namespace::new(struct_namespace),
        field: target.field_name.into_owned(),
        struct_field: target.struct_field_name.into_owned(),
        kind,
        rule: rule.to_owned(),
        reason: reason.to_owned(),
        params,
    })
}

fn push_nested_errors(errors: &mut Vec<FieldError>, target: FieldTarget<'_>, nested: Error) {
    if let Some(fields) = nested.into_fields() {
        for error in fields {
            errors.push(nested_field_error(target.clone(), error));
        }
    }
}

fn nested_field_error(target: FieldTarget<'_>, error: FieldError) -> FieldError {
    let parent_namespace = namespace_for(&target.type_name, &target.field_name);
    let parent_struct_namespace = namespace_for(&target.type_name, &target.struct_field_name);
    let namespace = nested_namespace(&parent_namespace, error.namespace().as_str());
    let struct_namespace =
        nested_namespace(&parent_struct_namespace, error.struct_namespace().as_str());

    FieldError::new(FieldErrorParts {
        namespace: Namespace::new(namespace),
        struct_namespace: Namespace::new(struct_namespace),
        field: error.field().to_owned(),
        struct_field: error.struct_field().to_owned(),
        kind: error.kind(),
        rule: error.rule().to_owned(),
        reason: error.reason().to_owned(),
        params: error.params().clone(),
    })
}

fn nested_namespace(parent: &str, child: &str) -> String {
    let relative = child
        .split_once('.')
        .map(|(_, relative)| relative)
        .unwrap_or(child);

    if relative.is_empty() {
        parent.to_owned()
    } else {
        format!("{parent}.{relative}")
    }
}

fn namespace_for(type_name: &str, field_name: &str) -> String {
    if type_name.is_empty() {
        field_name.to_owned()
    } else {
        format!("{type_name}.{field_name}")
    }
}
