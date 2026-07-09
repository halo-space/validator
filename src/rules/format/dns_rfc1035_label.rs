use std::sync::OnceLock;

use regex::Regex;

use crate::{Field, Rule};

#[derive(Debug)]
pub struct DnsRfc1035Label;

impl Rule for DnsRfc1035Label {
    fn check(&self, field: &Field<'_>) -> Result<bool, crate::Error> {
        Ok(field.value().string().is_some_and(|value| {
            let value = value.as_ref();
            value.len() <= 63 && pattern().is_match(value)
        }))
    }
}

fn pattern() -> &'static Regex {
    static PATTERN: OnceLock<Regex> = OnceLock::new();
    PATTERN.get_or_init(|| {
        Regex::new(r"^[a-z]([-a-z0-9]*[a-z0-9])?$").expect("dns_rfc1035_label regex must compile")
    })
}
