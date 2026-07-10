use crate::{Field, Rule};
use url::Url as ParsedUrl;

#[derive(Debug)]
pub struct Url;

impl Rule for Url {
    fn check(&self, field: &Field<'_>) -> Result<bool, crate::Error> {
        Ok(field
            .value()
            .string()
            .is_some_and(|value| parse(value.as_ref()).is_some()))
    }
}

pub(super) fn parse(value: &str) -> Option<ParsedUrl> {
    if value.is_empty() || value.chars().any(char::is_whitespace) {
        return None;
    }

    let parsed = ParsedUrl::parse(value).ok()?;
    if parsed.scheme() == "file" {
        return (!parsed.path().is_empty() && parsed.path() != "/").then_some(parsed);
    }

    let has_fragment = parsed
        .fragment()
        .is_some_and(|fragment| !fragment.is_empty());
    let has_opaque = parsed.cannot_be_a_base() && !parsed.path().is_empty();
    (parsed.has_host() || has_fragment || has_opaque).then_some(parsed)
}
