use std::sync::OnceLock;

use regex::Regex;

use crate::{Field, Rule};

#[derive(Debug)]
pub struct Cmyk;

impl Rule for Cmyk {
    fn check(&self, field: &Field<'_>) -> bool {
        field
            .value()
            .string()
            .is_some_and(|value| cmyk_regex().is_match(value.as_ref()))
    }
}

fn cmyk_regex() -> &'static Regex {
    static CMYK: OnceLock<Regex> = OnceLock::new();
    CMYK.get_or_init(|| {
        Regex::new(
            r"^cmyk\((?:100|[1-9]?[0-9])%\s*,\s*(?:100|[1-9]?[0-9])%\s*,\s*(?:100|[1-9]?[0-9])%\s*,\s*(?:100|[1-9]?[0-9])%\)$",
        )
        .expect("cmyk regex must compile")
    })
}
