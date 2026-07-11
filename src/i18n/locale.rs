use std::collections::BTreeMap;
use std::sync::Arc;

use serde::Deserialize;

use crate::{Error, FieldError};

use super::template::Template;
use super::translator::Context;

#[derive(Clone, Default)]
/// Translation templates and field labels for one locale identifier.
pub struct Locale {
    locale: String,
    rules: BTreeMap<String, Template>,
    fields: BTreeMap<String, String>,
}

impl Locale {
    /// Creates an empty locale resource.
    pub fn new(locale: impl Into<String>) -> Self {
        Self {
            locale: locale.into(),
            ..Self::default()
        }
    }

    /// Parses a locale resource from YAML text.
    pub fn from_yaml(yaml: impl AsRef<str>) -> Result<Self, Error> {
        let resource = serde_yaml_ng::from_str::<Resource>(yaml.as_ref()).map_err(invalid_error)?;
        Self::from_resource(resource)
    }

    /// Parses a locale resource from JSON text.
    pub fn from_json(json: impl AsRef<str>) -> Result<Self, Error> {
        let resource = serde_json::from_str::<Resource>(json.as_ref()).map_err(invalid_error)?;
        Self::from_resource(resource)
    }

    /// Returns this locale's identifier, such as `en` or `zh-CN`.
    pub fn locale(&self) -> &str {
        &self.locale
    }

    /// Adds a text template for one rule.
    pub fn rule(mut self, rule: impl Into<String>, template: impl Into<String>) -> Self {
        self.rules
            .insert(rule.into(), Template::Text(template.into()));
        self
    }

    /// Adds a dynamic template function for one rule.
    pub fn rule_fn<F>(mut self, rule: impl Into<String>, render: F) -> Self
    where
        F: for<'a> Fn(&Context<'a>) -> String + Send + Sync + 'static,
    {
        self.rules
            .insert(rule.into(), Template::Fn(Arc::new(render)));
        self
    }

    /// Adds an already constructed template for one rule.
    pub fn template(mut self, rule: impl Into<String>, template: Template) -> Self {
        self.rules.insert(rule.into(), template);
        self
    }

    /// Adds a display label for one field name.
    pub fn field(mut self, field: impl Into<String>, label: impl Into<String>) -> Self {
        self.fields.insert(field.into(), label.into());
        self
    }

    pub(super) fn merge(&mut self, other: Self) {
        self.rules.extend(other.rules);
        self.fields.extend(other.fields);
    }

    pub(super) fn template_for(&self, error: &FieldError) -> Option<&Template> {
        self.rules
            .get(error.rule())
            .or_else(|| self.rules.get(error.reason()))
    }

    pub(super) fn field_label<'a>(&'a self, error: &'a FieldError) -> &'a str {
        self.fields
            .get(error.field())
            .map(String::as_str)
            .unwrap_or_else(|| error.field())
    }

    fn from_resource(resource: Resource) -> Result<Self, Error> {
        let locale = resource
            .locale
            .ok_or_else(|| invalid("locale name is required"))?;
        if locale.trim().is_empty() {
            return Err(invalid("locale name is required"));
        }

        let mut locale = Self::new(locale);
        for (rule, template) in resource.rules.unwrap_or_default() {
            locale = locale.rule(rule, template);
        }
        for (field, label) in resource.fields.unwrap_or_default() {
            locale = locale.field(field, label);
        }

        Ok(locale)
    }

    #[cfg(test)]
    pub(super) fn has_rule(&self, name: &str) -> bool {
        self.rules.contains_key(name)
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Resource {
    locale: Option<String>,
    rules: Option<BTreeMap<String, String>>,
    fields: Option<BTreeMap<String, String>>,
}

fn invalid_error(error: impl std::error::Error) -> Error {
    invalid(error.to_string())
}

fn invalid(reason: impl Into<String>) -> Error {
    Error::InvalidData {
        reason: format!("invalid locale resource: {}", reason.into()),
    }
}
