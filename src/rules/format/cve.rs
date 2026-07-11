use std::sync::OnceLock;

use regex::Regex;

use crate::{Field, Rule};

#[derive(Debug)]
pub(crate) struct Cve;

impl Rule for Cve {
    fn check(&self, field: &Field<'_>) -> Result<bool, crate::Error> {
        Ok(field
            .value()
            .string()
            .is_some_and(|value| pattern().is_match(value.as_ref())))
    }
}

fn pattern() -> &'static Regex {
    static PATTERN: OnceLock<Regex> = OnceLock::new();
    PATTERN.get_or_init(|| {
        Regex::new(r"^CVE-(1999|2[0-9]{3})-(0[^0][0-9]{2}|0[0-9][^0][0-9]{1}|0[0-9]{2}[^0]|[1-9]{1}[0-9]{3,})$")
            .expect("cve regex must compile")
    })
}
