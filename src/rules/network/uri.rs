use crate::{Field, Rule};

#[derive(Debug)]
pub struct Uri;

impl Rule for Uri {
    fn check(&self, field: &Field<'_>) -> Result<bool, crate::Error> {
        Ok(field
            .value()
            .string()
            .is_some_and(|value| valid(value.as_ref())))
    }
}

fn valid(value: &str) -> bool {
    let value = value.split_once('#').map_or(value, |(value, _)| value);
    if value.is_empty() || value.chars().any(char::is_whitespace) {
        return false;
    }

    if value.starts_with('/') {
        return value.parse::<http::Uri>().is_ok();
    }

    url::Url::parse(value).is_ok()
}
