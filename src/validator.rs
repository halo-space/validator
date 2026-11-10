use std::sync::{Arc, RwLock};

use crate::core::{CAPACITY, Cache, Context, Expr, Fields, Group, Registry};
use crate::schema::{Schema, SchemaId, Tree};
use crate::target::FieldTarget;
use crate::traits::{Selective, Validate};
use crate::{Error, Namespace, Provider, Rule, Value};

#[path = "validator/cache.rs"]
mod cache;
#[path = "validator/derive.rs"]
mod derive;

/// Configures rules, aliases, and validation extensions.
pub struct Config {
    pub(crate) registry: Registry,
}

impl Config {
    /// Creates a configuration with all built-in rules and aliases.
    pub fn new() -> Self {
        Self::try_new().expect("default validator rules must be valid")
    }

    /// Creates a configuration without panicking if built-in registration fails.
    pub fn try_new() -> Result<Self, Error> {
        let mut registry = Registry::new();
        crate::rules::load(&mut registry)?;
        crate::rules::load_aliases(&mut registry)?;
        Ok(Self { registry })
    }

    /// Adds a custom rule to this configuration.
    pub fn rule<R>(mut self, name: impl Into<String>, rule: R) -> Result<Self, Error>
    where
        R: Rule + Send + Sync + 'static,
    {
        self.registry.rule(name, rule)?;
        Ok(self)
    }

    /// Adds an alias to this configuration.
    pub fn alias(mut self, name: impl Into<String>, expr: impl AsRef<str>) -> Result<Self, Error> {
        self.registry.alias(name, expr)?;
        Ok(self)
    }

    /// Applies a reusable rule provider to this configuration.
    pub fn provider<P: Provider>(self, provider: P) -> Result<Self, Error> {
        provider.register(self)
    }
}

impl Default for Config {
    fn default() -> Self {
        Self::new()
    }
}

/// Executes validation with configured rules, aliases, schemas, and caches.
pub struct Validator {
    pub(crate) config: Config,
    schema: Option<Schema>,
    expression_cache: RwLock<Cache<String, Arc<Vec<Expr>>>>,
    compiled_cache: RwLock<Cache<String, Arc<Group>>>,
    spec_cache: RwLock<Cache<cache::SpecKey, Arc<Group>>>,
    schema_cache: RwLock<Cache<SchemaId, Arc<Tree>>>,
}

impl Validator {
    /// Creates a validator with all built-in rules and aliases.
    pub fn new() -> Self {
        Self::try_new().expect("default validator rules must be valid")
    }

    /// Creates a validator with all built-in rules and aliases without
    /// panicking if the built-in registry configuration is invalid.
    pub fn try_new() -> Result<Self, Error> {
        Self::try_from(Config::try_new()?)
    }

    /// Creates a validator from a configured rule set.
    pub fn from(config: Config) -> Self {
        Self::try_from(config).expect("configured validator must be valid")
    }

    /// Creates a validator from a configured rule set without panicking.
    pub fn try_from(config: Config) -> Result<Self, Error> {
        Ok(Self {
            config,
            schema: None,
            expression_cache: RwLock::new(Cache::new(CAPACITY)),
            compiled_cache: RwLock::new(Cache::new(CAPACITY)),
            spec_cache: RwLock::new(Cache::new(CAPACITY)),
            schema_cache: RwLock::new(Cache::new(CAPACITY)),
        })
    }

    /// Creates a validator configured to execute the supplied dynamic schema.
    pub fn with_schema(schema: Schema) -> Self {
        let mut validator = Self::new();
        validator.schema = Some(schema);
        validator
    }

    /// Creates a schema-configured validator through the fallible constructor.
    pub fn try_with_schema(schema: Schema) -> Result<Self, Error> {
        let mut validator = Self::try_new()?;
        validator.schema = Some(schema);
        Ok(validator)
    }

    /// Validates a value through its [`Validate`] implementation.
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

    /// Registers a custom rule under the supplied name.
    pub fn rule<R>(mut self, name: impl Into<String>, rule: R) -> Result<Self, Error>
    where
        R: Rule + Send + Sync + 'static,
    {
        self.config.registry.rule(name, rule)?;
        Ok(self)
    }

    /// Registers an alias for a reusable rule expression.
    pub fn alias(mut self, name: impl Into<String>, expr: impl AsRef<str>) -> Result<Self, Error> {
        self.config.registry.alias(name, expr)?;
        Ok(self)
    }

    /// Validates one value against a runtime rule expression.
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

    /// Validates a JSON object against the configured schema.
    pub fn validate_map(&self, value: &serde_json::Value) -> Result<(), Error> {
        let schema = self.schema.as_ref().ok_or(Error::MissingSchema)?;
        self.validate_map_with(schema, value)
    }

    /// Validates a JSON object against a schema supplied for this call.
    pub fn validate_map_with(
        &self,
        schema: &Schema,
        value: &serde_json::Value,
    ) -> Result<(), Error> {
        let tree = self.schema_tree(schema)?;
        self.validate_data(&tree, value)
    }

    /// Serializes a value to JSON and validates it against the configured schema.
    pub fn validate_serde<T>(&self, value: &T) -> Result<(), Error>
    where
        T: serde::Serialize + ?Sized,
    {
        let schema = self.schema.as_ref().ok_or(Error::MissingSchema)?;
        self.validate_serde_with(schema, value)
    }

    /// Serializes a value to JSON and validates it against a schema supplied for this call.
    pub fn validate_serde_with<T>(&self, schema: &Schema, value: &T) -> Result<(), Error>
    where
        T: serde::Serialize + ?Sized,
    {
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
}

impl Default for Validator {
    fn default() -> Self {
        Self::new()
    }
}
