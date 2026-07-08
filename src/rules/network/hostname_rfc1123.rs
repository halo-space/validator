use crate::{Field, Rule};

#[derive(Debug)]
pub struct HostnameRfc1123;

impl Rule for HostnameRfc1123 {
    fn check(&self, field: &Field<'_>) -> Result<bool, crate::Error> {
        Ok(field
            .value()
            .string()
            .is_some_and(|value| is_hostname_rfc1123(value.as_ref())))
    }
}

fn is_hostname_rfc1123(value: &str) -> bool {
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
