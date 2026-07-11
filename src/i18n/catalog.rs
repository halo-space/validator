use std::collections::BTreeMap;

use super::locale::Locale;
use super::translator::Translator;
use super::{en, zh_cn};

#[derive(Clone, Default)]
/// A collection of locales with an optional fallback selection.
pub struct Catalog {
    locales: BTreeMap<String, Locale>,
    fallback: Option<String>,
}

impl Catalog {
    /// Creates an empty catalog.
    pub fn new() -> Self {
        Self::default()
    }

    /// Adds or merges the built-in Simplified Chinese locale.
    pub fn zh_cn(self) -> Self {
        self.use_locale(zh_cn::locale())
    }

    /// Adds or merges the built-in English locale.
    pub fn en(self) -> Self {
        self.use_locale(en::locale())
    }

    /// Adds or merges one locale resource.
    pub fn use_locale(mut self, locale: Locale) -> Self {
        self.locales
            .entry(locale.locale().to_owned())
            .and_modify(|current| current.merge(locale.clone()))
            .or_insert(locale);
        self
    }

    /// Sets the locale used when a requested locale is unavailable.
    pub fn fallback(mut self, locale: impl Into<String>) -> Self {
        self.fallback = Some(locale.into());
        self
    }

    /// Selects a translator for one locale identifier.
    pub fn locale(&self, locale: impl AsRef<str>) -> Translator<'_> {
        let selected = self.locales.get(locale.as_ref()).or_else(|| {
            self.fallback
                .as_deref()
                .and_then(|name| self.locales.get(name))
        });

        Translator::borrowed(selected)
    }
}
