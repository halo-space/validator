use std::sync::OnceLock;

use regex::Regex;

use crate::{Field, Rule};

#[derive(Debug)]
pub struct Rgba;

impl Rule for Rgba {
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
            r"^rgba\(\s*(?:(?:0|[1-9][0-9]?|1[0-9][0-9]|2[0-4][0-9]|25[0-5])\s*,\s*(?:0|[1-9][0-9]?|1[0-9][0-9]|2[0-4][0-9]|25[0-5])\s*,\s*(?:0|[1-9][0-9]?|1[0-9][0-9]|2[0-4][0-9]|25[0-5])|(?:0|[1-9][0-9]?|1[0-9][0-9]|2[0-4][0-9]|25[0-5])%\s*,\s*(?:0|[1-9][0-9]?|1[0-9][0-9]|2[0-4][0-9]|25[0-5])%\s*,\s*(?:0|[1-9][0-9]?|1[0-9][0-9]|2[0-4][0-9]|25[0-5])%)\s*,\s*(?:0(?:\.[0-9]+)?|1(?:\.0+)?)\s*\)$",
        )
        .expect("rgba regex must compile")
    })
}
