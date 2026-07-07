use regex::Regex;

use crate::{Field, Rule};

#[derive(Debug)]
pub struct RegexRule;

impl Rule for RegexRule {
    fn check(&self, field: &Field<'_>) -> bool {
        let Some(pattern) = field.params().get("pattern") else {
            return false;
        };
        let Some(value) = field.value().string() else {
            return false;
        };

        Regex::new(pattern).is_ok_and(|regex| regex.is_match(value.as_ref()))
    }
}
