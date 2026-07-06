use std::sync::OnceLock;

use regex::Regex;

use crate::{Field, Rule};

#[derive(Debug)]
pub struct Number;

impl Rule for Number {
    fn check(&self, field: &Field<'_>) -> bool {
        if field.value().number().is_some() {
            return true;
        }

        field
            .value()
            .string()
            .is_some_and(|value| number_regex().is_match(value.as_ref()))
    }
}

fn number_regex() -> &'static Regex {
    static NUMBER: OnceLock<Regex> = OnceLock::new();
    NUMBER.get_or_init(|| Regex::new(r"^[0-9]+$").expect("number regex must compile"))
}
