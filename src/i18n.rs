mod catalog;
mod en;
mod locale;
mod template;
mod translator;
mod zh_cn;

pub use catalog::Catalog;
pub use locale::Locale;
pub use template::{RenderFn, Template};
pub use translator::{Context, Message, Translator};

pub fn new() -> Catalog {
    Catalog::new()
}

pub fn zh_cn() -> Translator<'static> {
    Translator::owned(zh_cn::locale())
}

pub fn en() -> Translator<'static> {
    Translator::owned(en::locale())
}

#[cfg(test)]
mod tests {
    use crate::core::Registry;

    use super::*;

    #[test]
    fn built_in_locales_cover_default_rules_aliases_and_internal_errors() {
        let mut registry = Registry::new();
        crate::rules::load(&mut registry).expect("default rules must load");
        crate::rules::load_aliases(&mut registry).expect("default aliases must load");

        let mut names = registry
            .rule_names()
            .chain(registry.alias_names())
            .map(str::to_owned)
            .collect::<Vec<_>>();
        names.extend(crate::schema::internal_rule_names().map(str::to_owned));

        assert_locale_covers("zh-CN", &zh_cn::locale(), &names);
        assert_locale_covers("en", &en::locale(), &names);
    }

    fn assert_locale_covers(locale_name: &str, locale: &Locale, names: &[String]) {
        let missing = names
            .iter()
            .filter(|name| !locale.has_rule(name))
            .collect::<Vec<_>>();

        assert!(
            missing.is_empty(),
            "{locale_name} missing i18n templates: {missing:?}"
        );
    }
}
