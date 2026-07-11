use std::borrow::Cow;
use std::collections::BTreeMap;
use std::sync::Arc;

use serde::Deserialize;

use crate::{Error, FieldError, Kind, Namespace, Params};

use super::template::{self, Template};
use super::{en, zh_cn};

#[derive(Clone, Default)]
pub struct I18n {
    locales: BTreeMap<String, Locale>,
    fallback: Option<String>,
}

impl I18n {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn zh_cn(self) -> Self {
        self.use_locale(zh_cn::locale())
    }

    pub fn en(self) -> Self {
        self.use_locale(en::locale())
    }

    pub fn use_locale(mut self, locale: Locale) -> Self {
        self.locales
            .entry(locale.locale.clone())
            .and_modify(|current| current.merge(locale.clone()))
            .or_insert(locale);
        self
    }

    pub fn fallback(mut self, locale: impl Into<String>) -> Self {
        self.fallback = Some(locale.into());
        self
    }

    pub fn locale(&self, locale: impl AsRef<str>) -> Translator<'_> {
        let selected = self.locales.get(locale.as_ref()).or_else(|| {
            self.fallback
                .as_deref()
                .and_then(|name| self.locales.get(name))
        });

        Translator {
            locale: selected.map(Cow::Borrowed),
        }
    }
}

#[derive(Clone, Default)]
pub struct Locale {
    locale: String,
    rules: BTreeMap<String, Template>,
    fields: BTreeMap<String, String>,
}

impl Locale {
    pub fn new(locale: impl Into<String>) -> Self {
        Self {
            locale: locale.into(),
            ..Self::default()
        }
    }

    pub fn from_yaml(yaml: impl AsRef<str>) -> Result<Self, Error> {
        let resource =
            serde_yaml::from_str::<LocaleResource>(yaml.as_ref()).map_err(invalid_locale_error)?;
        Self::from_resource(resource)
    }

    pub fn from_json(json: impl AsRef<str>) -> Result<Self, Error> {
        let resource =
            serde_json::from_str::<LocaleResource>(json.as_ref()).map_err(invalid_locale_error)?;
        Self::from_resource(resource)
    }

    pub fn locale(&self) -> &str {
        &self.locale
    }

    pub fn rule(mut self, rule: impl Into<String>, template: impl Into<String>) -> Self {
        self.rules
            .insert(rule.into(), Template::Text(template.into()));
        self
    }

    pub fn rule_fn<F>(mut self, rule: impl Into<String>, render: F) -> Self
    where
        F: for<'a> Fn(&Context<'a>) -> String + Send + Sync + 'static,
    {
        self.rules
            .insert(rule.into(), Template::Fn(Arc::new(render)));
        self
    }

    pub fn template(mut self, rule: impl Into<String>, template: Template) -> Self {
        self.rules.insert(rule.into(), template);
        self
    }

    pub fn field(mut self, field: impl Into<String>, label: impl Into<String>) -> Self {
        self.fields.insert(field.into(), label.into());
        self
    }

    fn merge(&mut self, other: Locale) {
        self.rules.extend(other.rules);
        self.fields.extend(other.fields);
    }

    fn from_resource(resource: LocaleResource) -> Result<Self, Error> {
        let locale = resource
            .locale
            .ok_or_else(|| invalid_locale("locale name is required"))?;
        if locale.trim().is_empty() {
            return Err(invalid_locale("locale name is required"));
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

    fn template_for(&self, error: &FieldError) -> Option<&Template> {
        self.rules
            .get(error.rule())
            .or_else(|| self.rules.get(error.reason()))
    }

    fn field_label<'a>(&'a self, error: &'a FieldError) -> &'a str {
        self.fields
            .get(error.field())
            .map(String::as_str)
            .unwrap_or_else(|| error.field())
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct LocaleResource {
    locale: Option<String>,
    rules: Option<BTreeMap<String, String>>,
    fields: Option<BTreeMap<String, String>>,
}

pub struct Translator<'a> {
    locale: Option<Cow<'a, Locale>>,
}

impl Translator<'_> {
    pub fn render(&self, fields: &[FieldError]) -> Vec<Message> {
        fields.iter().map(|field| self.render_one(field)).collect()
    }

    fn render_one(&self, error: &FieldError) -> Message {
        let display_field = self
            .locale
            .as_deref()
            .map(|locale| locale.field_label(error))
            .unwrap_or_else(|| error.field());
        let context = Context {
            error,
            field: display_field,
        };
        let text = self
            .locale
            .as_deref()
            .and_then(|locale| locale.template_for(error))
            .map(|template| template::render(template, &context))
            .unwrap_or_else(|| template::default_text(error));

        Message {
            namespace: error.namespace().clone(),
            struct_namespace: error.struct_namespace().clone(),
            field: display_field.to_owned(),
            struct_field: error.struct_field().to_owned(),
            rule: error.rule().to_owned(),
            reason: error.reason().to_owned(),
            kind: error.kind(),
            params: error.params().clone(),
            text,
        }
    }
}

pub struct Context<'a> {
    error: &'a FieldError,
    field: &'a str,
}

impl<'a> Context<'a> {
    pub fn namespace(&self) -> &Namespace {
        self.error.namespace()
    }

    pub fn struct_namespace(&self) -> &Namespace {
        self.error.struct_namespace()
    }

    pub fn field(&self) -> &str {
        self.field
    }

    pub fn struct_field(&self) -> &str {
        self.error.struct_field()
    }

    pub fn rule(&self) -> &str {
        self.error.rule()
    }

    pub fn reason(&self) -> &str {
        self.error.reason()
    }

    pub fn kind(&self) -> Kind {
        self.error.kind()
    }

    pub fn params(&self) -> &Params {
        self.error.params()
    }

    pub fn param(&self, name: &str) -> Option<&str> {
        self.error.params().text(name)
    }

    pub fn param_list(&self, name: &str) -> Option<&[String]> {
        self.error.params().list(name)
    }

    pub fn param_pairs(&self, name: &str) -> Option<&[(String, String)]> {
        self.error.params().pairs(name)
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Message {
    pub namespace: Namespace,
    pub struct_namespace: Namespace,
    pub field: String,
    pub struct_field: String,
    pub rule: String,
    pub reason: String,
    pub kind: Kind,
    pub params: Params,
    pub text: String,
}

impl Locale {
    #[cfg(test)]
    pub(super) fn has_rule(&self, name: &str) -> bool {
        self.rules.contains_key(name)
    }
}

impl Translator<'static> {
    pub(super) fn owned(locale: Locale) -> Self {
        Self {
            locale: Some(Cow::Owned(locale)),
        }
    }
}

fn invalid_locale_error(error: impl std::error::Error) -> Error {
    invalid_locale(error.to_string())
}

fn invalid_locale(reason: impl Into<String>) -> Error {
    Error::InvalidData {
        reason: format!("invalid locale resource: {}", reason.into()),
    }
}
