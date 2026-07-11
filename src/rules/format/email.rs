use std::sync::OnceLock;

use regex::Regex;

use crate::{Field, Rule};

#[derive(Debug)]
pub(crate) struct Email;

impl Rule for Email {
    fn check(&self, field: &Field<'_>) -> Result<bool, crate::Error> {
        Ok(field
            .value()
            .string()
            .is_some_and(|value| valid(value.as_ref())))
    }
}

fn valid(value: &str) -> bool {
    if value.len() > 254 || !pattern().is_match(value) {
        return false;
    }

    let Some((local, domain)) = value.split_once('@') else {
        return false;
    };
    if local.len() > 64 || local.starts_with('.') || local.ends_with('.') || local.contains("..") {
        return false;
    }

    let domain = domain.strip_suffix('.').unwrap_or(domain);
    let mut labels = domain.split('.').peekable();
    let mut count = 0;
    while let Some(label) = labels.next() {
        count += 1;
        let last = labels.peek().is_none();
        if label.is_empty()
            || label.len() > 63
            || !label.chars().next().is_some_and(char::is_alphanumeric)
            || !label.chars().last().is_some_and(char::is_alphanumeric)
            || !label
                .chars()
                .all(|ch| ch.is_alphanumeric() || matches!(ch, '-' | '~'))
            || last && !label.chars().next().is_some_and(char::is_alphabetic)
        {
            return false;
        }
    }

    count >= 2
}

fn pattern() -> &'static Regex {
    static PATTERN: OnceLock<Regex> = OnceLock::new();
    PATTERN.get_or_init(|| {
        Regex::new(r"^[^\s@]+@[^\s@]+\.[^\s@]+$").expect("email regex must compile")
    })
}
