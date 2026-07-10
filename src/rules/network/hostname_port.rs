use crate::{Field, Rule};

#[derive(Debug)]
pub struct HostnamePort;

impl Rule for HostnamePort {
    fn check(&self, field: &Field<'_>) -> Result<bool, crate::Error> {
        Ok(field
            .value()
            .string()
            .is_some_and(|value| valid(value.as_ref())))
    }
}

fn valid(value: &str) -> bool {
    let Some((host, port)) = split_host_port(value) else {
        return false;
    };

    let Ok(port) = port.parse::<u16>() else {
        return false;
    };
    if port == 0 {
        return false;
    }

    host.is_empty() || super::host::valid_rfc1123(host)
}

fn split_host_port(value: &str) -> Option<(&str, &str)> {
    if let Some(rest) = value.strip_prefix('[') {
        let (host, rest) = rest.split_once(']')?;
        let port = rest.strip_prefix(':')?;
        return Some((host, port));
    }

    let (host, port) = value.rsplit_once(':')?;
    if host.contains(':') {
        return None;
    }
    Some((host, port))
}
