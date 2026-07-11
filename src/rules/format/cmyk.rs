use std::sync::OnceLock;

use regex::Regex;

use crate::{Field, Rule};

#[derive(Debug)]
pub(crate) struct Cmyk;

impl Rule for Cmyk {
    fn check(&self, field: &Field<'_>) -> Result<bool, crate::Error> {
        Ok(field
            .value()
            .string()
            .is_some_and(|value| pattern().is_match(value.as_ref())))
    }
}

fn pattern() -> &'static Regex {
    static PATTERN: OnceLock<Regex> = OnceLock::new();
    PATTERN.get_or_init(|| {
        Regex::new(
            r"^cmyk\((?:100|[1-9]?[0-9])%\s*,\s*(?:100|[1-9]?[0-9])%\s*,\s*(?:100|[1-9]?[0-9])%\s*,\s*(?:100|[1-9]?[0-9])%\)$",
        )
        .expect("cmyk regex must compile")
    })
}
