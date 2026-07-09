use std::net::IpAddr;

use crate::{Field, Rule};

#[derive(Debug)]
pub struct Origin;

impl Rule for Origin {
    fn check(&self, field: &Field<'_>) -> Result<bool, crate::Error> {
        Ok(field
            .value()
            .string()
            .is_some_and(|value| valid(value.as_ref())))
    }
}

fn valid(value: &str) -> bool {
    let Some((scheme, authority)) = value.split_once("://") else {
        return false;
    };

    if !scheme.eq_ignore_ascii_case("http") && !scheme.eq_ignore_ascii_case("https") {
        return false;
    }
    if authority.is_empty()
        || authority.contains(['/', '?', '#', '@'])
        || authority.chars().any(char::is_whitespace)
    {
        return false;
    }

    let Some((host, port)) = split_authority(authority) else {
        return false;
    };

    valid_host(host) && port.is_none_or(valid_port)
}

fn split_authority(authority: &str) -> Option<(&str, Option<&str>)> {
    if let Some(rest) = authority.strip_prefix('[') {
        let (host, rest) = rest.split_once(']')?;
        let port = match rest {
            "" => None,
            rest => Some(rest.strip_prefix(':')?),
        };
        return Some((host, port));
    }

    if authority.matches(':').count() > 1 {
        return None;
    }

    match authority.rsplit_once(':') {
        Some((host, port)) => Some((host, Some(port))),
        None => Some((authority, None)),
    }
}

fn valid_host(host: &str) -> bool {
    if host.is_empty() {
        return false;
    }

    host.parse::<IpAddr>().is_ok() || valid_hostname(host)
}

fn valid_hostname(host: &str) -> bool {
    if host.is_empty() || host.len() > 253 || host.starts_with('.') || host.ends_with('.') {
        return false;
    }

    host.split('.').all(|label| {
        !label.is_empty()
            && label.len() <= 63
            && !label.starts_with('-')
            && !label.ends_with('-')
            && label
                .bytes()
                .all(|byte| byte.is_ascii_alphanumeric() || byte == b'-')
    })
}

fn valid_port(port: &str) -> bool {
    !port.is_empty()
        && port.bytes().all(|byte| byte.is_ascii_digit())
        && port.parse::<u16>().is_ok_and(|port| port > 0)
}
