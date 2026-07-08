use std::sync::OnceLock;

use regex::Regex;

use crate::{Field, Rule};

#[derive(Debug)]
pub struct Rgb;

impl Rule for Rgb {
    fn check(&self, field: &Field<'_>) -> Result<bool, crate::Error> {
        Ok(field
            .value()
            .string()
            .is_some_and(|value| rgb_regex().is_match(value.as_ref())))
    }
}

fn rgb_regex() -> &'static Regex {
    static RGB: OnceLock<Regex> = OnceLock::new();
    RGB.get_or_init(|| {
        Regex::new(
            r"^rgb\(\s*(?:(?:0|[1-9][0-9]?|1[0-9][0-9]|2[0-4][0-9]|25[0-5])\s*,\s*(?:0|[1-9][0-9]?|1[0-9][0-9]|2[0-4][0-9]|25[0-5])\s*,\s*(?:0|[1-9][0-9]?|1[0-9][0-9]|2[0-4][0-9]|25[0-5])|(?:0|[1-9][0-9]?|1[0-9][0-9]|2[0-4][0-9]|25[0-5])%\s*,\s*(?:0|[1-9][0-9]?|1[0-9][0-9]|2[0-4][0-9]|25[0-5])%\s*,\s*(?:0|[1-9][0-9]?|1[0-9][0-9]|2[0-4][0-9]|25[0-5])%)\s*\)$",
        )
        .expect("rgb regex must compile")
    })
}
