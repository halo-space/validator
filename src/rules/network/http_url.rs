use crate::{Field, Rule};

#[derive(Debug)]
pub struct HttpUrl;

impl Rule for HttpUrl {
    fn check(&self, field: &Field<'_>) -> Result<bool, crate::Error> {
        Ok(field
            .value()
            .string()
            .is_some_and(|value| valid_http(value.as_ref())))
    }
}

#[derive(Debug)]
pub struct HttpsUrl;

impl Rule for HttpsUrl {
    fn check(&self, field: &Field<'_>) -> Result<bool, crate::Error> {
        Ok(field
            .value()
            .string()
            .is_some_and(|value| valid_https(value.as_ref())))
    }
}

fn valid_http(value: &str) -> bool {
    super::url::has_scheme_and_host(value)
        && super::url::scheme(value).is_some_and(|scheme| {
            scheme.eq_ignore_ascii_case("http") || scheme.eq_ignore_ascii_case("https")
        })
}

fn valid_https(value: &str) -> bool {
    super::url::has_scheme_and_host(value)
        && super::url::scheme(value).is_some_and(|scheme| scheme.eq_ignore_ascii_case("https"))
}
