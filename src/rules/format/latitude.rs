use std::sync::OnceLock;

use regex::Regex;

use crate::{Field, Rule};

#[derive(Debug)]
pub(crate) struct Latitude;

impl Rule for Latitude {
    fn check(&self, field: &Field<'_>) -> Result<bool, crate::Error> {
        Ok(value(field).is_some_and(|value| pattern().is_match(&value)))
    }
}

fn value(field: &Field<'_>) -> Option<String> {
    if let Some(value) = field.value().string() {
        return Some(value.into_owned());
    }
    if let Some(value) = field.value().int() {
        return Some(value.to_string());
    }
    if let Some(value) = field.value().uint() {
        return Some(value.to_string());
    }
    field.value().float().map(float_string)
}

fn float_string(value: f64) -> String {
    if !value.is_finite() {
        return value.to_string();
    }

    let mut text = format!("{value:.15}");
    while text.contains('.') && text.ends_with('0') {
        text.pop();
    }
    if text.ends_with('.') {
        text.pop();
    }
    text
}

fn pattern() -> &'static Regex {
    static PATTERN: OnceLock<Regex> = OnceLock::new();
    PATTERN.get_or_init(|| {
        Regex::new(r"^[-+]?([1-8]?[0-9](\.[0-9]+)?|90(\.0+)?)$")
            .expect("latitude regex must compile")
    })
}
