use crate::{Field, Rule};

#[derive(Debug)]
pub struct Hostname;

impl Rule for Hostname {
    fn check(&self, field: &Field<'_>) -> Result<bool, crate::Error> {
        Ok(field
            .value()
            .string()
            .is_some_and(|value| is_hostname(value.as_ref())))
    }
}

#[derive(Debug)]
pub struct Fqdn;

impl Rule for Fqdn {
    fn check(&self, field: &Field<'_>) -> Result<bool, crate::Error> {
        Ok(field
            .value()
            .string()
            .is_some_and(|value| is_hostname(value.as_ref()) && value.contains('.')))
    }
}

fn is_hostname(value: &str) -> bool {
    if value.is_empty() || value.len() > 253 || value.starts_with('.') || value.ends_with('.') {
        return false;
    }

    value.split('.').all(is_label)
}

fn is_label(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 63
        && !value.starts_with('-')
        && !value.ends_with('-')
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || byte == b'-')
}
