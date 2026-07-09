use std::sync::OnceLock;

use regex::Regex;

use crate::{Field, Rule};

#[derive(Debug)]
pub struct DataUri;

impl Rule for DataUri {
    fn check(&self, field: &Field<'_>) -> Result<bool, crate::Error> {
        Ok(field
            .value()
            .string()
            .is_some_and(|value| valid(value.as_ref())))
    }
}

fn valid(value: &str) -> bool {
    let Some((header, payload)) = value.split_once(',') else {
        return false;
    };

    header_pattern().is_match(header) && payload_pattern().is_match(payload)
}

fn header_pattern() -> &'static Regex {
    static PATTERN: OnceLock<Regex> = OnceLock::new();
    PATTERN.get_or_init(|| {
        Regex::new(r"^data:((?:\w+/(?:([^;]|;[^;]).)+)?)")
            .expect("datauri header regex must compile")
    })
}

fn payload_pattern() -> &'static Regex {
    static PATTERN: OnceLock<Regex> = OnceLock::new();
    PATTERN.get_or_init(|| {
        Regex::new(
            r"^(?:[A-Za-z0-9+/]{4})*(?:[A-Za-z0-9+/]{2}==|[A-Za-z0-9+/]{3}=|[A-Za-z0-9+/]{4})$",
        )
        .expect("datauri payload regex must compile")
    })
}
