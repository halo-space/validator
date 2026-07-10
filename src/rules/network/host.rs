use crate::{Field, Rule};

#[derive(Debug)]
pub struct Hostname;

impl Rule for Hostname {
    fn check(&self, field: &Field<'_>) -> Result<bool, crate::Error> {
        Ok(field
            .value()
            .string()
            .is_some_and(|value| valid_rfc952(value.as_ref())))
    }
}

#[derive(Debug)]
pub struct Fqdn;

impl Rule for Fqdn {
    fn check(&self, field: &Field<'_>) -> Result<bool, crate::Error> {
        Ok(field
            .value()
            .string()
            .is_some_and(|value| valid_fqdn(value.as_ref())))
    }
}

fn valid_rfc952(value: &str) -> bool {
    value
        .bytes()
        .next()
        .is_some_and(|byte| byte.is_ascii_alphabetic())
        && valid_hostname(value)
}

pub(super) fn valid_rfc1123(value: &str) -> bool {
    valid_hostname(value)
}

fn valid_hostname(value: &str) -> bool {
    if value.is_empty() || value.starts_with('.') || value.ends_with('.') {
        return false;
    }

    value.split('.').all(is_label)
}

fn valid_fqdn(value: &str) -> bool {
    let value = value.strip_suffix('.').unwrap_or(value);
    if value.is_empty() || !value.contains('.') {
        return false;
    }

    let mut labels = value.split('.').peekable();
    while let Some(label) = labels.next() {
        let last = labels.peek().is_none();
        if !is_fqdn_label(label, last) {
            return false;
        }
    }

    true
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

fn is_fqdn_label(value: &str, tld: bool) -> bool {
    let Some(first) = value.bytes().next() else {
        return false;
    };

    value.len() <= 63
        && if tld {
            first.is_ascii_alphabetic()
        } else {
            first.is_ascii_alphanumeric()
        }
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || byte == b'-')
}
