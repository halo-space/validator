use std::sync::OnceLock;

use regex::Regex;

use crate::{Field, Rule};

#[derive(Debug)]
pub struct Hsla;

impl Rule for Hsla {
    fn check(&self, field: &Field<'_>) -> Result<bool, crate::Error> {
        Ok(field
            .value()
            .string()
            .is_some_and(|value| hsla_regex().is_match(value.as_ref())))
    }
}

fn hsla_regex() -> &'static Regex {
    static HSLA: OnceLock<Regex> = OnceLock::new();
    HSLA.get_or_init(|| {
        Regex::new(
            r"^hsla\(\s*(?:0|[1-9][0-9]?|[12][0-9][0-9]|3[0-5][0-9]|360)\s*,\s*(?:0|[1-9][0-9]?|100)%\s*,\s*(?:0|[1-9][0-9]?|100)%\s*,\s*(?:0(?:\.[0-9]+)?|1(?:\.0+)?)\s*\)$",
        )
        .expect("hsla regex must compile")
    })
}
