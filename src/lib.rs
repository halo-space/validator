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

use self::core::{Aliases, Context, Expr, FieldErrorParts, Fields, Group, Rules, parse_expression};
pub use self::core::{
    Error, Field, FieldError, FloatKind, IntKind, Kind, Namespace, Number, Params, Rule, UintKind,
    Value,
};
pub use self::schema::Schema;
use self::schema::Tree;
pub use validator_derive::Validate;

#[doc(hidden)]
pub mod __private {
    pub use crate::core::{Access, Context, FieldRef};
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
    expression_cache: RwLock<BTreeMap<String, Arc<Vec<Expr>>>>,
    compiled_cache: RwLock<BTreeMap<(u64, String), Arc<Group>>>,
    schema_cache: RwLock<BTreeMap<(schema::SchemaId, u64), Arc<Tree>>>,
}

impl Validator {
    pub fn new() -> Self {
        let mut rules = Rules::new();
        crate::rules::load(&mut rules).expect("default validator rules must be valid");

        let mut aliases = Aliases::new();
        crate::rules::load_aliases(&mut aliases).expect("default validator aliases must be valid");

        Self {
            rules,
            aliases,
            schema: None,
            generation: 0,
            expression_cache: RwLock::new(BTreeMap::new()),
            compiled_cache: RwLock::new(BTreeMap::new()),
            schema_cache: RwLock::new(BTreeMap::new()),
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

        let mut errors = Vec::new();
        let context = Context::new();
        tree.validate(&context, &mut errors, value)?;

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
        self.compiled_cache
            .write()
            .expect("compiled cache lock must not be poisoned")
            .clear();
        self.schema_cache
            .write()
            .expect("schema cache lock must not be poisoned")
            .clear();
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
        let group = Arc::new(Group::compile(exprs.as_ref(), &self.rules, &self.aliases)?);
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

        let tree = Arc::new(schema.compile(&self.rules, &self.aliases)?);
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

    #[doc(hidden)]
    pub fn __validate_rule<V: Value>(
        &self,
        errors: &mut Vec<FieldError>,
        target: FieldTarget<'_>,
        value: &V,
        rule: &str,
        params: Params,
        context: &Context,
    ) -> Result<(), Error> {
        self.__validate_rule_with_display(
            errors,
            target,
            value,
            RuleDisplay::same(rule),
            params,
            context,
        )
    }

    #[doc(hidden)]
    pub fn __validate_field_rule<'a, V, F>(
        &self,
        errors: &mut Vec<FieldError>,
        target: FieldTarget<'_>,
        value: &V,
        rule: &str,
        params: Params,
        fields: F,
    ) where
        V: Value,
        F: Fn(&str) -> Option<&'a dyn Value> + 'a,
    {
        let fields = &fields as &Fields<'a>;
        if field_rule_passes(value, &params, Some(fields), rule) {
            return;
        }

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
        context: &Context,
    ) -> Result<(), Error> {
        let Some(exprs) = self.aliases.get(alias) else {
            return Err(Error::UnknownAlias {
                name: alias.to_owned(),
            });
        };
        let group = Group::compile_alias(exprs, &self.rules, &self.aliases)?;
        group.execute_alias(errors, target, value, alias, context)
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

    fn __validate_rule_with_display<V: Value>(
        &self,
        errors: &mut Vec<FieldError>,
        target: FieldTarget<'_>,
        value: &V,
        display: RuleDisplay<'_>,
        params: Params,
        context: &Context,
    ) -> Result<(), Error> {
        if !self.__rule_passes(target.clone(), value, display.reason, &params, context)? {
            errors.push(field_error(
                target,
                value.kind(),
                display.rule,
                display.reason,
                params,
            ));
        }
        Ok(())
    }

    fn __rule_passes<V: Value>(
        &self,
        target: FieldTarget<'_>,
        value: &V,
        reason: &str,
        params: &Params,
        context: &Context,
    ) -> Result<bool, Error> {
        if reason == "omitempty" {
            return Ok(true);
        }

        if reason != "required" && value.is_none() {
            return Ok(true);
        }

        let Some(handler) = self.rules.get(reason) else {
            return Ok(false);
        };

        let namespace = Namespace::new(namespace_for(&target.type_name, &target.field_name));
        let struct_namespace =
            Namespace::new(namespace_for(&target.type_name, &target.struct_field_name));
        let field = Field::with_context(
            &namespace,
            &struct_namespace,
            target.field_name.as_ref(),
            target.struct_field_name.as_ref(),
            params,
            value,
            context,
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

#[derive(Clone, Copy)]
struct RuleDisplay<'a> {
    rule: &'a str,
    reason: &'a str,
}

impl<'a> RuleDisplay<'a> {
    fn new(rule: &'a str, reason: &'a str) -> Self {
        Self { rule, reason }
    }

    fn same(name: &'a str) -> Self {
        Self::new(name, name)
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

pub(crate) fn is_field_rule(name: &str) -> bool {
    field_rule(name).is_some()
}

fn compare_field(params: &Params) -> Option<&str> {
    params.get("compare").or_else(|| params.get("value"))
}

pub(crate) fn field_targets<'a>(rule: &str, params: &'a Params) -> Vec<&'a str> {
    match field_rule(rule) {
        Some(FieldRule::Relation(_) | FieldRule::Contains | FieldRule::Excludes) => {
            compare_field(params).into_iter().collect()
        }
        Some(
            FieldRule::RequiredIf
            | FieldRule::RequiredUnless
            | FieldRule::SkipUnless
            | FieldRule::ExcludedIf
            | FieldRule::ExcludedUnless,
        ) => params.iter().map(|(field, _)| field).collect(),
        Some(
            FieldRule::RequiredWith
            | FieldRule::RequiredWithAll
            | FieldRule::RequiredWithout
            | FieldRule::RequiredWithoutAll
            | FieldRule::ExcludedWith
            | FieldRule::ExcludedWithAll
            | FieldRule::ExcludedWithout
            | FieldRule::ExcludedWithoutAll,
        ) => field_list(params),
        None => Vec::new(),
    }
}

#[derive(Clone, Copy)]
enum FieldRule {
    Relation(FieldRelation),
    Contains,
    Excludes,
    RequiredIf,
    RequiredUnless,
    SkipUnless,
    RequiredWith,
    RequiredWithAll,
    RequiredWithout,
    RequiredWithoutAll,
    ExcludedIf,
    ExcludedUnless,
    ExcludedWith,
    ExcludedWithAll,
    ExcludedWithout,
    ExcludedWithoutAll,
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

fn field_rule(rule: &str) -> Option<FieldRule> {
    match rule {
        "eq_field" => Some(FieldRule::Relation(FieldRelation::Eq)),
        "ne_field" => Some(FieldRule::Relation(FieldRelation::Ne)),
        "gt_field" => Some(FieldRule::Relation(FieldRelation::Gt)),
        "gte_field" => Some(FieldRule::Relation(FieldRelation::Gte)),
        "lt_field" => Some(FieldRule::Relation(FieldRelation::Lt)),
        "lte_field" => Some(FieldRule::Relation(FieldRelation::Lte)),
        "fieldcontains" => Some(FieldRule::Contains),
        "fieldexcludes" => Some(FieldRule::Excludes),
        "required_if" => Some(FieldRule::RequiredIf),
        "required_unless" => Some(FieldRule::RequiredUnless),
        "skip_unless" => Some(FieldRule::SkipUnless),
        "required_with" => Some(FieldRule::RequiredWith),
        "required_with_all" => Some(FieldRule::RequiredWithAll),
        "required_without" => Some(FieldRule::RequiredWithout),
        "required_without_all" => Some(FieldRule::RequiredWithoutAll),
        "excluded_if" => Some(FieldRule::ExcludedIf),
        "excluded_unless" => Some(FieldRule::ExcludedUnless),
        "excluded_with" => Some(FieldRule::ExcludedWith),
        "excluded_with_all" => Some(FieldRule::ExcludedWithAll),
        "excluded_without" => Some(FieldRule::ExcludedWithout),
        "excluded_without_all" => Some(FieldRule::ExcludedWithoutAll),
        _ => None,
    }
}

pub(crate) fn field_rule_passes<V: Value>(
    value: &V,
    params: &Params,
    fields: Option<&Fields<'_>>,
    rule: &str,
) -> bool {
    let Some(rule) = field_rule(rule) else {
        return false;
    };

    if matches!(
        rule,
        FieldRule::Relation(_) | FieldRule::Contains | FieldRule::Excludes
    ) && value.is_none()
    {
        return true;
    }

    if matches!(
        rule,
        FieldRule::RequiredIf
            | FieldRule::RequiredUnless
            | FieldRule::SkipUnless
            | FieldRule::ExcludedIf
            | FieldRule::ExcludedUnless
    ) && params.is_empty()
    {
        return value.required();
    }

    if field_list_rule(rule) && field_list(params).is_empty() {
        return true;
    }

    let Some(fields) = fields else {
        return true;
    };

    match rule {
        FieldRule::Relation(relation) => {
            let field_value = compare_field(params).and_then(fields);
            compare_field_rule(value, field_value, FieldRule::Relation(relation))
        }
        FieldRule::Contains => {
            let field_value = compare_field(params).and_then(fields);
            compare_field_rule(value, field_value, FieldRule::Contains)
        }
        FieldRule::Excludes => {
            let field_value = compare_field(params).and_then(fields);
            compare_field_rule(value, field_value, FieldRule::Excludes)
        }
        FieldRule::RequiredIf => {
            if pair_fields_match(params, fields) {
                value.required()
            } else {
                true
            }
        }
        FieldRule::SkipUnless => {
            if pair_fields_match(params, fields) {
                value.required()
            } else {
                true
            }
        }
        FieldRule::RequiredUnless => {
            if any_pair_field_matches(params, fields) {
                true
            } else {
                value.required()
            }
        }
        FieldRule::RequiredWith => {
            if field_list(params)
                .into_iter()
                .any(|field| field_is_present(fields(field)))
            {
                value.required()
            } else {
                true
            }
        }
        FieldRule::RequiredWithAll => {
            if field_list(params)
                .into_iter()
                .all(|field| field_is_present(fields(field)))
            {
                value.required()
            } else {
                true
            }
        }
        FieldRule::RequiredWithout => {
            if field_list(params)
                .into_iter()
                .any(|field| !field_is_present(fields(field)))
            {
                value.required()
            } else {
                true
            }
        }
        FieldRule::RequiredWithoutAll => {
            if field_list(params)
                .into_iter()
                .all(|field| !field_is_present(fields(field)))
            {
                value.required()
            } else {
                true
            }
        }
        FieldRule::ExcludedIf => {
            if pair_fields_match(params, fields) {
                !value.required()
            } else {
                true
            }
        }
        FieldRule::ExcludedUnless => {
            if any_pair_field_matches(params, fields) {
                true
            } else {
                !value.required()
            }
        }
        FieldRule::ExcludedWith => {
            if field_list(params)
                .into_iter()
                .any(|field| field_is_present(fields(field)))
            {
                !value.required()
            } else {
                true
            }
        }
        FieldRule::ExcludedWithAll => {
            if field_list(params)
                .into_iter()
                .all(|field| field_is_present(fields(field)))
            {
                !value.required()
            } else {
                true
            }
        }
        FieldRule::ExcludedWithout => {
            if field_list(params)
                .into_iter()
                .any(|field| !field_is_present(fields(field)))
            {
                !value.required()
            } else {
                true
            }
        }
        FieldRule::ExcludedWithoutAll => {
            if field_list(params)
                .into_iter()
                .all(|field| !field_is_present(fields(field)))
            {
                !value.required()
            } else {
                true
            }
        }
    }
}

fn compare_field_rule(value: &dyn Value, field_value: Option<&dyn Value>, rule: FieldRule) -> bool {
    let Some(field_value) = field_value else {
        return matches!(rule, FieldRule::Excludes);
    };
    if field_value.is_none() {
        return matches!(rule, FieldRule::Excludes);
    }

    match rule {
        FieldRule::Relation(relation) => values_satisfy(value, field_value, relation),
        FieldRule::Contains => strings_contain(value, field_value).unwrap_or(false),
        FieldRule::Excludes => strings_contain(value, field_value).is_none_or(|contains| !contains),
        FieldRule::RequiredIf
        | FieldRule::RequiredUnless
        | FieldRule::SkipUnless
        | FieldRule::RequiredWith
        | FieldRule::RequiredWithAll
        | FieldRule::RequiredWithout
        | FieldRule::RequiredWithoutAll
        | FieldRule::ExcludedIf
        | FieldRule::ExcludedUnless
        | FieldRule::ExcludedWith
        | FieldRule::ExcludedWithAll
        | FieldRule::ExcludedWithout
        | FieldRule::ExcludedWithoutAll => false,
    }
}

fn field_list_rule(rule: FieldRule) -> bool {
    matches!(
        rule,
        FieldRule::RequiredWith
            | FieldRule::RequiredWithAll
            | FieldRule::RequiredWithout
            | FieldRule::RequiredWithoutAll
            | FieldRule::ExcludedWith
            | FieldRule::ExcludedWithAll
            | FieldRule::ExcludedWithout
            | FieldRule::ExcludedWithoutAll
    )
}

fn field_list(params: &Params) -> Vec<&str> {
    params
        .get("fields")
        .into_iter()
        .flat_map(|fields| fields.split(','))
        .map(str::trim)
        .filter(|field| !str::is_empty(field))
        .collect()
}

fn field_is_present(value: Option<&dyn Value>) -> bool {
    value.is_some_and(Value::required)
}

fn pair_fields_match(params: &Params, fields: &Fields<'_>) -> bool {
    params
        .iter()
        .all(|(field, expected)| field_matches_value(fields(field), expected))
}

fn any_pair_field_matches(params: &Params, fields: &Fields<'_>) -> bool {
    params
        .iter()
        .any(|(field, expected)| field_matches_value(fields(field), expected))
}

fn field_matches_value(value: Option<&dyn Value>, expected: &str) -> bool {
    let Some(value) = value else {
        return false;
    };

    if value.is_none() {
        return matches!(expected, "nil" | "null");
    }

    match value.kind() {
        Kind::String => value.string().is_some_and(|value| value == expected),
        Kind::Bool => expected
            .parse::<bool>()
            .is_ok_and(|expected| value.boolean() == Some(expected)),
        Kind::Int(_) => expected
            .parse::<i128>()
            .is_ok_and(|expected| value.int() == Some(expected)),
        Kind::Uint(_) => expected
            .parse::<u128>()
            .is_ok_and(|expected| value.uint() == Some(expected)),
        Kind::Float(_) => expected
            .parse::<f64>()
            .is_ok_and(|expected| value.float() == Some(expected)),
        Kind::Vec | Kind::Array | Kind::Slice | Kind::Map => expected
            .parse::<usize>()
            .is_ok_and(|expected| value.len() == Some(expected)),
        Kind::Time | Kind::Option | Kind::Other => false,
    }
}

fn strings_contain(left: &dyn Value, right: &dyn Value) -> Option<bool> {
    Some(left.string()?.contains(right.string()?.as_ref()))
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
        Kind::Time => Some(left.time()? == right.time()?),
        Kind::Vec | Kind::Array | Kind::Slice | Kind::Map => Some(left.len()? == right.len()?),
        Kind::Option | Kind::Other => None,
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
        Kind::Time => left.time()?.partial_cmp(&right.time()?),
        Kind::Bool | Kind::Option | Kind::Other => None,
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
