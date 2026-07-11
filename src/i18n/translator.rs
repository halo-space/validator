use std::borrow::Cow;

use crate::{FieldError, Kind, Namespace, Params};

use super::locale::Locale;
use super::template;

/// Renders field errors with one selected locale.
pub struct Translator<'a> {
    locale: Option<Cow<'a, Locale>>,
}

impl<'a> Translator<'a> {
    /// Renders all field errors into localized messages.
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

/// Structured data available to dynamic locale templates.
pub struct Context<'a> {
    error: &'a FieldError,
    field: &'a str,
}

impl Context<'_> {
    /// Returns the runtime namespace.
    pub fn namespace(&self) -> &Namespace {
        self.error.namespace()
    }

    /// Returns the Rust struct namespace.
    pub fn struct_namespace(&self) -> &Namespace {
        self.error.struct_namespace()
    }

    /// Returns the display field label selected by the locale.
    pub fn field(&self) -> &str {
        self.field
    }

    /// Returns the Rust struct field name.
    pub fn struct_field(&self) -> &str {
        self.error.struct_field()
    }

    /// Returns the displayed rule or alias name.
    pub fn rule(&self) -> &str {
        self.error.rule()
    }

    /// Returns the underlying failed rule name.
    pub fn reason(&self) -> &str {
        self.error.reason()
    }

    /// Returns the failed value kind.
    pub fn kind(&self) -> Kind {
        self.error.kind()
    }

    /// Returns all bound rule parameters.
    pub fn params(&self) -> &Params {
        self.error.params()
    }

    /// Returns one text parameter.
    pub fn param(&self, name: &str) -> Option<&str> {
        self.error.params().text(name)
    }

    /// Returns one list parameter.
    pub fn param_list(&self, name: &str) -> Option<&[String]> {
        self.error.params().list(name)
    }

    /// Returns one name/value-pairs parameter.
    pub fn param_pairs(&self, name: &str) -> Option<&[(String, String)]> {
        self.error.params().pairs(name)
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
/// One localized validation message with its structured error context.
pub struct Message {
    /// Runtime namespace.
    pub namespace: Namespace,
    /// Rust struct namespace.
    pub struct_namespace: Namespace,
    /// Display field label.
    pub field: String,
    /// Rust struct field name.
    pub struct_field: String,
    /// Displayed rule or alias name.
    pub rule: String,
    /// Underlying failed rule name.
    pub reason: String,
    /// Failed value kind.
    pub kind: Kind,
    /// Bound rule parameters.
    pub params: Params,
    /// Rendered localized text.
    pub text: String,
}
