use std::sync::OnceLock;

use regex::Regex;

use crate::{Field, Rule};

#[derive(Debug)]
pub(crate) struct AlphanumUnicode;

#[async_trait::async_trait(?Send)]
impl Rule for AlphanumUnicode {
    async fn check(&self, field: &Field<'_>) -> Result<bool, crate::Error> {
        Ok(field
            .value()
            .string()
            .is_some_and(|value| pattern().is_match(value.as_ref())))
    }
}

fn pattern() -> &'static Regex {
    static PATTERN: OnceLock<Regex> = OnceLock::new();
    PATTERN
        .get_or_init(|| Regex::new(r"^[\p{L}\p{N}]+$").expect("alphanumunicode regex must compile"))
}
