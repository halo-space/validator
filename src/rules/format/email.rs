use std::sync::OnceLock;

use regex::Regex;

use crate::{Field, Rule};

#[derive(Debug)]
pub struct Email;

impl Rule for Email {
    fn check(&self, field: &Field<'_>) -> Result<bool, crate::Error> {
        Ok(field
            .value()
            .string()
            .is_some_and(|value| email_regex().is_match(value.as_ref())))
    }
}

fn email_regex() -> &'static Regex {
    static EMAIL: OnceLock<Regex> = OnceLock::new();
    EMAIL.get_or_init(|| {
        Regex::new(r"^[^\s@]+@[^\s@]+\.[^\s@]+$").expect("email regex must compile")
    })
}
