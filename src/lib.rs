mod core;
pub mod i18n;
mod rules;
mod schema;
pub mod valid;

use std::borrow::Cow;
use std::cmp::Ordering;
use std::collections::BTreeMap;
use std::fmt;
use std::sync::{Arc, RwLock};

use self::core::{Aliases, FieldErrorParts, RuleGroup, Rules, parse_rule_expression};
pub use self::core::{
    Error, Field, FieldError, FloatKind, IntKind, Kind, Namespace, Number, Params, Rule, UintKind,
    Value,
};
pub use self::schema::Schema;
pub use validator_derive::Validate;

#[doc(hidden)]
pub mod __private {
    pub use crate::core::{Access, FieldRef};
}

pub mod prelude {
    pub use crate::{
        Error, Field, FieldError, FloatKind, IntKind, Kind, Namespace, Number, Params, Rule,
        Schema, Validate, Validator, Value,
    };
}

pub struct Validator {
    rules: Rules,
    aliases: Aliases,
    schema: Option<Schema>,
    generation: u64,
    expression_cache: RwLock<BTreeMap<String, Arc<Vec<RuleGroup>>>>,
    schema_checked_generation: RwLock<Option<u64>>,
}

impl Validator {
    pub fn new() -> Self {
        let mut rules = Rules::new();
        crate::rules::load_rules(&mut rules).expect("default validator rules must be valid");

        let mut aliases = Aliases::new();
        crate::rules::load_aliases(&mut aliases).expect("default validator aliases must be valid");

        Self {
            rules,
            aliases,
            schema: None,
            generation: 0,
            expression_cache: RwLock::new(BTreeMap::new()),
            schema_checked_generation: RwLock::new(None),
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

    pub fn rule<R>(mut self, name: impl Into<String>, rule: R) -> Result<Self, Error>
    where
        R: Rule + Send + Sync + 'static,
    {
        self.rules.insert(name, rule)?;
        self.bump_generation();
        Ok(self)
    }

    pub fn alias(mut self, name: impl Into<String>, expr: impl AsRef<str>) -> Result<Self, Error> {
        self.aliases.insert(name, expr)?;
        self.bump_generation();
        Ok(self)
    }

    pub fn value<V: Value>(&self, value: &V, rules: impl AsRef<str>) -> Result<(), Error> {
        let groups = self.cached_rule_expression(rules.as_ref())?;
        self.ensure_direct_value_rules(groups.as_ref())?;
        let mut errors = Vec::new();
        let target = FieldTarget::value();
        self.validate_rule_groups(&mut errors, target, value, groups.as_ref());

        if errors.is_empty() {
            Ok(())
        } else {
            Err(Error::failed(errors))
        }
    }

    pub fn validate_map(&self, value: &serde_json::Value) -> Result<(), Error> {
        let schema = self.schema.as_ref().ok_or(Error::MissingSchema)?;

        self.ensure_schema_rules(schema)?;

        let mut errors = Vec::new();
        schema.validate(self, &mut errors, value);

        if errors.is_empty() {
            Ok(())
        } else {
            Err(Error::failed(errors))
        }
    }

    pub fn validate_serde<T>(&self, value: &T) -> Result<(), Error>
    where
        T: serde::Serialize + ?Sized,
    {
        let data = serde_json::to_value(value).map_err(|error| Error::InvalidData {
            reason: error.to_string(),
        })?;

        self.validate_map(&data)
    }

    fn bump_generation(&mut self) {
        self.generation = self.generation.wrapping_add(1);
        *self
            .schema_checked_generation
            .write()
            .expect("schema verification cache lock must not be poisoned") = None;
    }

    fn cached_rule_expression(&self, expression: &str) -> Result<Arc<Vec<RuleGroup>>, Error> {
        if let Some(groups) = self
            .expression_cache
            .read()
            .expect("rule expression cache lock must not be poisoned")
            .get(expression)
            .cloned()
        {
            return Ok(groups);
        }

        let groups = Arc::new(parse_rule_expression(expression)?);
        let mut cache = self
            .expression_cache
            .write()
            .expect("rule expression cache lock must not be poisoned");
        if let Some(groups) = cache.get(expression).cloned() {
            return Ok(groups);
        }

        cache.insert(expression.to_owned(), groups.clone());
        Ok(groups)
    }

    fn ensure_schema_rules(&self, schema: &Schema) -> Result<(), Error> {
        let checked_generation = *self
            .schema_checked_generation
            .read()
            .expect("schema verification cache lock must not be poisoned");
        if checked_generation == Some(self.generation) {
            return Ok(());
        }

        schema.ensure_rules(self)?;

        *self
            .schema_checked_generation
            .write()
            .expect("schema verification cache lock must not be poisoned") = Some(self.generation);
        Ok(())
    }

    pub(crate) fn ensure_rule_groups(&self, groups: &[RuleGroup]) -> Result<(), Error> {
        for group in groups {
            if let Some(spec) = group.single() {
                self.ensure_rule(spec.name())?;
                continue;
            }

            if let Some(alternatives) = group.alternatives() {
                for spec in alternatives {
                    self.ensure_rule(spec.name())?;
                }
            }
        }

        Ok(())
    }

    fn ensure_direct_value_rules(&self, groups: &[RuleGroup]) -> Result<(), Error> {
        self.ensure_rule_groups(groups)
    }

    fn ensure_rule(&self, name: &str) -> Result<(), Error> {
        if name == "omitempty" || self.rules.contains(name) {
            return Ok(());
        }

        if self.aliases.contains(name) {
            return self.ensure_alias_rules(name);
        }

        Err(Error::UnknownRule {
            name: name.to_owned(),
        })
    }

    fn ensure_alias_rules(&self, alias: &str) -> Result<(), Error> {
        let Some(groups) = self.aliases.get(alias) else {
            return Err(Error::UnknownAlias {
                name: alias.to_owned(),
            });
        };

        for group in groups {
            if let Some(spec) = group.single() {
                self.ensure_alias_rule(spec.name())?;
                continue;
            }

            if let Some(alternatives) = group.alternatives() {
                for spec in alternatives {
                    self.ensure_alias_rule(spec.name())?;
                }
            }
        }

        Ok(())
    }

    fn ensure_alias_rule(&self, name: &str) -> Result<(), Error> {
        if name == "omitempty" || self.rules.contains(name) {
            return Ok(());
        }

        Err(Error::UnknownRule {
            name: name.to_owned(),
        })
    }

    pub(crate) fn validate_rule_groups<V: Value>(
        &self,
        errors: &mut Vec<FieldError>,
        target: FieldTarget<'_>,
        value: &V,
        groups: &[RuleGroup],
    ) {
        for group in groups {
            if let Some(spec) = group.single() {
                if spec.name() == "omitempty" {
                    if self.__skip_empty(value) {
                        return;
                    }
                    continue;
                }

                if !self.rules.contains(spec.name()) && self.aliases.contains(spec.name()) {
                    self.__validate_alias(errors, target.clone(), value, spec.name());
                    continue;
                }

                self.__validate_rule_with_display(
                    errors,
                    target.clone(),
                    value,
                    spec.name(),
                    spec.name(),
                    spec.params().clone(),
                );
                continue;
            }

            if let Some(alternatives) = group.alternatives() {
                if alternatives.iter().any(|spec| {
                    self.direct_spec_passes(target.clone(), value, spec.name(), spec.params())
                }) {
                    continue;
                }

                let reason = alternatives
                    .iter()
                    .map(|spec| spec.name())
                    .collect::<Vec<_>>()
                    .join("|");
                errors.push(field_error(
                    target.clone(),
                    value.kind(),
                    &reason,
                    &reason,
                    Params::new(),
                ));
            }
        }
    }

    pub(crate) fn validate_rule_groups_with_compare<'a, V, F>(
        &self,
        errors: &mut Vec<FieldError>,
        target: FieldTarget<'_>,
        value: &V,
        groups: &[RuleGroup],
        compare_field: F,
    ) where
        V: Value,
        F: Fn(&str) -> Option<&'a dyn Value>,
    {
        for group in groups {
            if let Some(spec) = group.single() {
                if spec.name() == "omitempty" {
                    if self.__skip_empty(value) {
                        return;
                    }
                    continue;
                }

                if is_cross_field_rule(spec.name()) {
                    let compare = compare_param(spec.params()).unwrap_or_default();
                    self.__validate_compare_field(
                        errors,
                        target.clone(),
                        value,
                        compare_field(compare),
                        spec.name(),
                        compare,
                    );
                    continue;
                }

                if !self.rules.contains(spec.name()) && self.aliases.contains(spec.name()) {
                    self.__validate_alias(errors, target.clone(), value, spec.name());
                    continue;
                }

                self.__validate_rule_with_display(
                    errors,
                    target.clone(),
                    value,
                    spec.name(),
                    spec.name(),
                    spec.params().clone(),
                );
                continue;
            }

            if let Some(alternatives) = group.alternatives() {
                if alternatives.iter().any(|spec| {
                    self.spec_passes_with_compare(
                        target.clone(),
                        value,
                        spec.name(),
                        spec.params(),
                        &compare_field,
                    )
                }) {
                    continue;
                }

                let reason = alternatives
                    .iter()
                    .map(|spec| spec.name())
                    .collect::<Vec<_>>()
                    .join("|");
                errors.push(field_error(
                    target.clone(),
                    value.kind(),
                    &reason,
                    &reason,
                    Params::new(),
                ));
            }
        }
    }

    #[doc(hidden)]
    pub fn __validate_rule<V: Value>(
        &self,
        errors: &mut Vec<FieldError>,
        target: FieldTarget<'_>,
        value: &V,
        rule: &str,
        params: Params,
    ) {
        self.__validate_rule_with_display(errors, target, value, rule, rule, params);
    }

    #[doc(hidden)]
    pub fn __validate_compare_field<V: Value>(
        &self,
        errors: &mut Vec<FieldError>,
        target: FieldTarget<'_>,
        value: &V,
        compare_value: Option<&dyn Value>,
        rule: &str,
        compare: &str,
    ) {
        if compare_field_passes(value, compare_value, rule) {
            return;
        }

        let mut params = Params::new();
        params.insert("compare", compare);
        errors.push(field_error(target, value.kind(), rule, rule, params));
    }

    #[doc(hidden)]
    pub fn __validate_unique_items<'a, I, V>(
        &self,
        errors: &mut Vec<FieldError>,
        target: FieldTarget<'_>,
        kind: Kind,
        items: I,
    ) where
        I: IntoIterator<Item = &'a V>,
        V: Value + 'a,
    {
        let items = items
            .into_iter()
            .map(|item| item as &dyn Value)
            .collect::<Vec<_>>();

        if crate::rules::values_are_unique(items) {
            return;
        }

        errors.push(field_error(target, kind, "unique", "unique", Params::new()));
    }

    #[doc(hidden)]
    pub fn __validate_alias<V: Value>(
        &self,
        errors: &mut Vec<FieldError>,
        target: FieldTarget<'_>,
        value: &V,
        alias: &str,
    ) {
        let Some(specs) = self.aliases.get(alias) else {
            errors.push(field_error(
                target.clone(),
                value.kind(),
                alias,
                "alias",
                Params::new(),
            ));
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
                    target.clone(),
                    value,
                    alias,
                    spec.name(),
                    spec.params().clone(),
                );
                continue;
            }

            if let Some(alternatives) = group.alternatives() {
                if alternatives.iter().any(|spec| {
                    self.__rule_passes(target.clone(), value, spec.name(), spec.params())
                }) {
                    continue;
                }

                let reason = alternatives
                    .iter()
                    .map(|spec| spec.name())
                    .collect::<Vec<_>>()
                    .join("|");
                errors.push(field_error(
                    target.clone(),
                    value.kind(),
                    alias,
                    &reason,
                    Params::new(),
                ));
            }
        }
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
    ) {
        if let Err(nested) = value.validate(self) {
            push_nested_errors(errors, target, nested);
        }
    }

    #[doc(hidden)]
    pub fn __validate_nested_option<T: Validate>(
        &self,
        errors: &mut Vec<FieldError>,
        target: FieldTarget<'_>,
        value: &Option<T>,
    ) {
        if let Some(value) = value {
            self.__validate_nested(errors, target, value);
        }
    }

    #[doc(hidden)]
    pub fn __valid<'a>(
        &self,
        type_name: &'a str,
        errors: &'a mut Vec<FieldError>,
    ) -> valid::Valid<'a> {
        valid::Valid::new(type_name, errors)
    }

    fn __validate_rule_with_display<V: Value>(
        &self,
        errors: &mut Vec<FieldError>,
        target: FieldTarget<'_>,
        value: &V,
        rule: &str,
        reason: &str,
        params: Params,
    ) {
        if !self.__rule_passes(target.clone(), value, reason, &params) {
            errors.push(field_error(target, value.kind(), rule, reason, params));
        }
    }

    fn __rule_passes<V: Value>(
        &self,
        target: FieldTarget<'_>,
        value: &V,
        reason: &str,
        params: &Params,
    ) -> bool {
        if reason == "omitempty" {
            return true;
        }

        if reason != "required" && value.is_none() {
            return true;
        }

        let Some(handler) = self.rules.get(reason) else {
            return false;
        };

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
        );

        handler.check(&field)
    }

    fn direct_spec_passes<V: Value>(
        &self,
        target: FieldTarget<'_>,
        value: &V,
        name: &str,
        params: &Params,
    ) -> bool {
        if self.rules.contains(name) || name == "omitempty" {
            return self.__rule_passes(target.clone(), value, name, params);
        }

        if self.aliases.contains(name) {
            let mut errors = Vec::new();
            self.__validate_alias(&mut errors, target.clone(), value, name);
            return errors.is_empty();
        }

        false
    }

    fn spec_passes_with_compare<'a, V, F>(
        &self,
        target: FieldTarget<'_>,
        value: &V,
        name: &str,
        params: &Params,
        compare_field: &F,
    ) -> bool
    where
        V: Value,
        F: Fn(&str) -> Option<&'a dyn Value>,
    {
        if is_cross_field_rule(name) {
            let compare = compare_param(params).unwrap_or_default();
            return compare_field_passes(value, compare_field(compare), name);
        }

        self.direct_spec_passes(target, value, name, params)
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
        Self {
            type_name: self.type_name.clone(),
            field_name: Cow::Owned(format!("{}[\"{key}\"]", self.field_name)),
            struct_field_name: Cow::Owned(format!("{}[\"{key}\"]", self.struct_field_name)),
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

pub(crate) fn is_cross_field_rule(name: &str) -> bool {
    compare_relation(name).is_some()
}

fn compare_param(params: &Params) -> Option<&str> {
    params.get("compare").or_else(|| params.get("value"))
}

#[derive(Clone, Copy)]
enum FieldRelation {
    Eq,
    Ne,
    Gt,
    Gte,
    Lt,
    Lte,
}

fn compare_relation(rule: &str) -> Option<FieldRelation> {
    match rule {
        "eq_field" => Some(FieldRelation::Eq),
        "ne_field" => Some(FieldRelation::Ne),
        "gt_field" => Some(FieldRelation::Gt),
        "gte_field" => Some(FieldRelation::Gte),
        "lt_field" => Some(FieldRelation::Lt),
        "lte_field" => Some(FieldRelation::Lte),
        _ => None,
    }
}

fn compare_field_passes<V: Value>(
    value: &V,
    compare_value: Option<&dyn Value>,
    rule: &str,
) -> bool {
    if value.is_none() {
        return true;
    }

    let Some(compare_value) = compare_value else {
        return false;
    };
    if compare_value.is_none() {
        return false;
    }

    compare_relation(rule).is_some_and(|relation| values_satisfy(value, compare_value, relation))
}

fn values_satisfy(left: &dyn Value, right: &dyn Value, relation: FieldRelation) -> bool {
    if left.kind() != right.kind() {
        return false;
    }

    match relation {
        FieldRelation::Eq => values_equal(left, right).unwrap_or(false),
        FieldRelation::Ne => values_equal(left, right).is_some_and(|equal| !equal),
        FieldRelation::Gt | FieldRelation::Gte | FieldRelation::Lt | FieldRelation::Lte => {
            values_cmp(left, right).is_some_and(|ordering| ordering_matches(ordering, relation))
        }
    }
}

fn values_equal(left: &dyn Value, right: &dyn Value) -> Option<bool> {
    match left.kind() {
        Kind::String => Some(left.string()? == right.string()?),
        Kind::Bool => Some(left.boolean()? == right.boolean()?),
        Kind::Int(_) => Some(left.int()? == right.int()?),
        Kind::Uint(_) => Some(left.uint()? == right.uint()?),
        Kind::Float(_) => Some(left.float()? == right.float()?),
        Kind::Vec | Kind::Array | Kind::Slice | Kind::Map => Some(left.len()? == right.len()?),
        Kind::Option | Kind::Time | Kind::Other => None,
    }
}

fn values_cmp(left: &dyn Value, right: &dyn Value) -> Option<Ordering> {
    match left.kind() {
        Kind::String | Kind::Vec | Kind::Array | Kind::Slice | Kind::Map => {
            Some(left.len()?.cmp(&right.len()?))
        }
        Kind::Int(_) => Some(left.int()?.cmp(&right.int()?)),
        Kind::Uint(_) => Some(left.uint()?.cmp(&right.uint()?)),
        Kind::Float(_) => left.float()?.partial_cmp(&right.float()?),
        Kind::Bool | Kind::Option | Kind::Time | Kind::Other => None,
    }
}

fn ordering_matches(ordering: Ordering, relation: FieldRelation) -> bool {
    match relation {
        FieldRelation::Eq => ordering == Ordering::Equal,
        FieldRelation::Ne => ordering != Ordering::Equal,
        FieldRelation::Gt => ordering == Ordering::Greater,
        FieldRelation::Gte => matches!(ordering, Ordering::Greater | Ordering::Equal),
        FieldRelation::Lt => ordering == Ordering::Less,
        FieldRelation::Lte => matches!(ordering, Ordering::Less | Ordering::Equal),
    }
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
