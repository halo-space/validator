use crate::{Field, Rule};

#[derive(Debug)]
pub(crate) struct Http;

impl Rule for Http {
    fn check(&self, field: &Field<'_>) -> Result<bool, crate::Error> {
        Ok(field
            .value()
            .string()
            .is_some_and(|value| valid_http(value.as_ref())))
    }
}

#[derive(Debug)]
pub(crate) struct Https;

impl Rule for Https {
    fn check(&self, field: &Field<'_>) -> Result<bool, crate::Error> {
        Ok(field
            .value()
            .string()
            .is_some_and(|value| valid_https(value.as_ref())))
    }
}

fn valid_http(value: &str) -> bool {
    super::url::parse(value)
        .is_some_and(|url| url.has_host() && matches!(url.scheme(), "http" | "https"))
}

fn valid_https(value: &str) -> bool {
    super::url::parse(value).is_some_and(|url| url.has_host() && url.scheme() == "https")
}
