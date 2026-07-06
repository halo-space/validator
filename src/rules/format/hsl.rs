use std::sync::OnceLock;

use regex::Regex;

use crate::{Field, Rule};

#[derive(Debug)]
pub struct Hsl;

impl Rule for Hsl {
    fn check(&self, field: &Field<'_>) -> bool {
        field
            .value()
            .string()
            .is_some_and(|value| hsl_regex().is_match(value.as_ref()))
    }
}

fn hsl_regex() -> &'static Regex {
    static HSL: OnceLock<Regex> = OnceLock::new();
    HSL.get_or_init(|| {
        Regex::new(
            r"^hsl\(\s*(?:0|[1-9][0-9]?|[12][0-9][0-9]|3[0-5][0-9]|360)\s*,\s*(?:0|[1-9][0-9]?|100)%\s*,\s*(?:0|[1-9][0-9]?|100)%\s*\)$",
        )
        .expect("hsl regex must compile")
    })
}
