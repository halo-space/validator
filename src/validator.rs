use std::sync::atomic::{AtomicU64, Ordering};
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
    cache: Limits,
}

#[derive(Clone, Copy, Debug)]
struct Limits {
    expression: usize,
    compiled: usize,
    spec: usize,
    schema: usize,
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
        Ok(Self {
            registry,
            cache: Limits {
                expression: CAPACITY,
                compiled: CAPACITY,
                spec: CAPACITY,
                schema: CAPACITY,
            },
        })
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

    /// Sets the capacity shared by expression, group, spec, and schema caches.
    pub fn cache(mut self, capacity: usize) -> Self {
        self.cache = Limits {
            expression: capacity,
            compiled: capacity,
            spec: capacity,
            schema: capacity,
        };
        self
    }

    /// Sets the expression parsing cache capacity.
    pub fn expression_cache(mut self, capacity: usize) -> Self {
        self.cache.expression = capacity;
        self
    }

    /// Sets the compiled expression cache capacity.
    pub fn compiled_cache(mut self, capacity: usize) -> Self {
        self.cache.compiled = capacity;
        self
    }

    /// Sets the single-spec compilation cache capacity.
    pub fn spec_cache(mut self, capacity: usize) -> Self {
        self.cache.spec = capacity;
        self
    }

    /// Sets the Schema tree cache capacity.
    pub fn schema_cache(mut self, capacity: usize) -> Self {
        self.cache.schema = capacity;
        self
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
    metrics: Metrics,
}

/// A point-in-time snapshot of validator cache activity.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct Stats {
    /// Expression parsing cache hits.
    pub expression_hits: u64,
    /// Expression parsing cache misses.
    pub expression_misses: u64,
    /// Compiled expression cache hits.
    pub compiled_hits: u64,
    /// Compiled expression cache misses.
    pub compiled_misses: u64,
    /// Spec compilation cache hits.
    pub spec_hits: u64,
    /// Spec compilation cache misses.
    pub spec_misses: u64,
    /// Schema tree cache hits.
    pub schema_hits: u64,
    /// Schema tree cache misses.
    pub schema_misses: u64,
}

#[derive(Default)]
pub(crate) struct Metrics {
    expression_hits: AtomicU64,
    expression_misses: AtomicU64,
    compiled_hits: AtomicU64,
    compiled_misses: AtomicU64,
    spec_hits: AtomicU64,
    spec_misses: AtomicU64,
    schema_hits: AtomicU64,
    schema_misses: AtomicU64,
}

impl Metrics {
    pub(crate) fn hit_expression(&self) {
        self.expression_hits.fetch_add(1, Ordering::Relaxed);
    }

    pub(crate) fn miss_expression(&self) {
        self.expression_misses.fetch_add(1, Ordering::Relaxed);
    }

    pub(crate) fn hit_compiled(&self) {
        self.compiled_hits.fetch_add(1, Ordering::Relaxed);
    }

    pub(crate) fn miss_compiled(&self) {
        self.compiled_misses.fetch_add(1, Ordering::Relaxed);
    }

    pub(crate) fn hit_spec(&self) {
        self.spec_hits.fetch_add(1, Ordering::Relaxed);
    }

    pub(crate) fn miss_spec(&self) {
        self.spec_misses.fetch_add(1, Ordering::Relaxed);
    }

    pub(crate) fn hit_schema(&self) {
        self.schema_hits.fetch_add(1, Ordering::Relaxed);
    }

    pub(crate) fn miss_schema(&self) {
        self.schema_misses.fetch_add(1, Ordering::Relaxed);
    }

    pub(crate) fn snapshot(&self) -> Stats {
        let load = |counter: &AtomicU64| counter.load(Ordering::Relaxed);
        Stats {
            expression_hits: load(&self.expression_hits),
            expression_misses: load(&self.expression_misses),
            compiled_hits: load(&self.compiled_hits),
            compiled_misses: load(&self.compiled_misses),
            spec_hits: load(&self.spec_hits),
            spec_misses: load(&self.spec_misses),
            schema_hits: load(&self.schema_hits),
            schema_misses: load(&self.schema_misses),
        }
    }
}

impl Validator {
    /// Creates a validator with all built-in rules and aliases.
    pub fn new() -> Self {
        Self::try_new().expect("default validator rules must be valid")
    }

    /// Creates a validator with all built-in rules and aliases without
    /// panicking if the built-in registry configuration is invalid.
    pub fn try_new() -> Result<Self, Error> {
        Ok(Self::from(Config::try_new()?))
    }

    /// Creates a validator from a configured rule set.
    pub fn from(config: Config) -> Self {
        Self::with_config(config)
    }

    fn with_config(config: Config) -> Self {
        let cache = config.cache;
        Self {
            config,
            schema: None,
            expression_cache: RwLock::new(Cache::new(cache.expression)),
            compiled_cache: RwLock::new(Cache::new(cache.compiled)),
            spec_cache: RwLock::new(Cache::new(cache.spec)),
            schema_cache: RwLock::new(Cache::new(cache.schema)),
            metrics: Metrics::default(),
        }
    }

    /// Returns a snapshot of cache hits and misses.
    pub fn stats(&self) -> Stats {
        self.metrics.snapshot()
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
    pub async fn validate<T: Validate>(&self, value: &T) -> Result<(), Error> {
        value.validate(self).await
    }

    /// Validates only the selected relative Rust field paths.
    pub async fn partial<T, I, S>(&self, value: &T, fields: I) -> Result<(), Error>
    where
        T: Validate + Selective,
        I: IntoIterator<Item = S>,
        S: AsRef<str>,
    {
        let fields = Fields::new(fields);
        let context = Context::partial(&fields);
        let result = value.__validate_with_context(self, &context).await;
        fields.verify()?;
        result
    }

    /// Validates every field except the selected relative Rust field paths.
    pub async fn except<T, I, S>(&self, value: &T, fields: I) -> Result<(), Error>
    where
        T: Validate + Selective,
        I: IntoIterator<Item = S>,
        S: AsRef<str>,
    {
        let fields = Fields::new(fields);
        let context = Context::except(&fields);
        let result = value.__validate_with_context(self, &context).await;
        fields.verify()?;
        result
    }

    /// Validates fields for which the relative Namespace predicate returns true.
    pub async fn filter<T, F>(&self, value: &T, filter: F) -> Result<(), Error>
    where
        T: Validate + Selective,
        F: Fn(&Namespace) -> bool,
    {
        let context = Context::filter(&filter);
        value.__validate_with_context(self, &context).await
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
    pub async fn value<V: Value>(&self, value: &V, rules: impl AsRef<str>) -> Result<(), Error> {
        let group = self.compile(rules.as_ref())?;
        let mut errors = Vec::new();
        let target = FieldTarget::value();
        let context = Context::new();
        group.execute(&mut errors, target, value, &context).await?;

        if errors.is_empty() {
            Ok(())
        } else {
            Err(Error::failed(errors))
        }
    }

    /// Validates a JSON object against the configured schema.
    pub async fn check_map(&self, value: &serde_json::Value) -> Result<(), Error> {
        let schema = self.schema.as_ref().ok_or(Error::MissingSchema)?;
        let tree = self.schema_tree(schema)?;
        self.validate_data(&tree, value).await
    }

    /// Serializes a value to JSON and validates it against the configured schema.
    pub async fn check_serde<T>(&self, value: &T) -> Result<(), Error>
    where
        T: serde::Serialize + ?Sized,
    {
        let schema = self.schema.as_ref().ok_or(Error::MissingSchema)?;
        let tree = self.schema_tree(schema)?;
        let data = serde_json::to_value(value).map_err(|error| Error::InvalidData {
            reason: error.to_string(),
        })?;
        self.validate_data(&tree, &data).await
    }

    async fn validate_data(&self, tree: &Tree, value: &serde_json::Value) -> Result<(), Error> {
        let mut errors = Vec::new();
        let context = Context::new();
        tree.validate(&context, &mut errors, value).await?;

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
