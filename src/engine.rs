use std::sync::{Arc, RwLock, RwLockReadGuard, RwLockWriteGuard};

use crate::core::{CAPACITY, Cache, Context, Expr, Fields, Group, RawParams, Registry};
use crate::schema::{self, Schema, Tree};
use crate::target::FieldTarget;
use crate::traits::{Selective, Validate};
use crate::{Error, Namespace, Rule, Value};

mod cache;
mod derive_api;

trait RwLockExt<T> {
    fn read_unpoisoned(&self) -> RwLockReadGuard<'_, T>;
    fn write_unpoisoned(&self) -> RwLockWriteGuard<'_, T>;
}

impl<T> RwLockExt<T> for RwLock<T> {
    fn read_unpoisoned(&self) -> RwLockReadGuard<'_, T> {
        self.read().unwrap_or_else(|error| error.into_inner())
    }

    fn write_unpoisoned(&self) -> RwLockWriteGuard<'_, T> {
        self.write().unwrap_or_else(|error| error.into_inner())
    }
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
}

impl Default for Validator {
    fn default() -> Self {
        Self::new()
    }
}
