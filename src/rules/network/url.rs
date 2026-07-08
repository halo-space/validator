use crate::{Field, Rule};

#[derive(Debug)]
pub struct UrlRule;

impl Rule for UrlRule {
    fn check(&self, field: &Field<'_>) -> Result<bool, crate::Error> {
        Ok(field
            .value()
            .string()
            .is_some_and(|value| has_scheme_and_host(value.as_ref())))
    }
}

pub(super) fn has_scheme_and_host(value: &str) -> bool {
    let Some((scheme, rest)) = value.split_once("://") else {
        return false;
    };

    !scheme.is_empty()
        && !rest
            .split(['/', '?', '#'])
            .next()
            .unwrap_or_default()
            .is_empty()
}

pub(super) fn scheme(value: &str) -> Option<&str> {
    value.split_once("://").map(|(scheme, _)| scheme)
}
