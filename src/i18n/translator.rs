use std::borrow::Cow;

use crate::{FieldError, Kind, Namespace, Params};

use super::locale::Locale;
use super::template;

pub struct Translator<'a> {
    locale: Option<Cow<'a, Locale>>,
}

impl<'a> Translator<'a> {
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

    pub(super) fn borrowed(locale: Option<&'a Locale>) -> Self {
        Self {
            locale: locale.map(Cow::Borrowed),
        }
    }
}

impl Translator<'static> {
    pub(super) fn owned(locale: Locale) -> Self {
        Self {
            locale: Some(Cow::Owned(locale)),
        }
    }
}

pub struct Context<'a> {
    error: &'a FieldError,
    field: &'a str,
}

impl Context<'_> {
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
