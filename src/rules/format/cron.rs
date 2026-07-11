use std::sync::OnceLock;

use regex::Regex;

use crate::{Field, Rule};

#[derive(Debug)]
pub(crate) struct Cron;

impl Rule for Cron {
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
        Regex::new(r"^((@(annually|yearly|monthly|weekly|daily|hourly|reboot))|(@every ([0-9]+(ns|us|\x{00B5}s|ms|s|m|h))+)|(([A-Za-z0-9*?][A-Za-z0-9*?/,#L-]+|[*?0-9])( +([A-Za-z0-9*?][A-Za-z0-9*?/,#L-]+|[*?0-9])){4,6}))$")
            .expect("cron regex must compile")
    })
}
