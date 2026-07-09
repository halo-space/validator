use std::sync::OnceLock;

use regex::Regex;

use crate::{Field, Rule};

#[derive(Debug)]
pub struct Numeric;

impl Rule for Numeric {
    fn check(&self, field: &Field<'_>) -> Result<bool, crate::Error> {
        if field.value().number().is_some() {
            return Ok(true);
        }

        Ok(field
            .value()
            .string()
            .is_some_and(|value| pattern().is_match(value.as_ref())))
    }
}

fn pattern() -> &'static Regex {
    static PATTERN: OnceLock<Regex> = OnceLock::new();
    PATTERN.get_or_init(|| {
        Regex::new(r"^[-+]?[0-9]+(?:\.[0-9]+)?$").expect("numeric regex must compile")
    })
}
