use std::sync::OnceLock;

use regex::Regex;

use crate::{Field, Rule};

#[derive(Debug)]
pub struct Numeric;

impl Rule for Numeric {
    fn check(&self, field: &Field<'_>) -> bool {
        if field.value().number().is_some() {
            return true;
        }

        field
            .value()
            .string()
            .is_some_and(|value| numeric_regex().is_match(value.as_ref()))
    }
}

fn numeric_regex() -> &'static Regex {
    static NUMERIC: OnceLock<Regex> = OnceLock::new();
    NUMERIC.get_or_init(|| {
        Regex::new(r"^[-+]?[0-9]+(?:\.[0-9]+)?$").expect("numeric regex must compile")
    })
}
