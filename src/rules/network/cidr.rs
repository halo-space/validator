use std::net::{IpAddr, Ipv4Addr, Ipv6Addr};

use crate::{Field, Rule};

#[derive(Debug)]
pub struct Cidr;

impl Rule for Cidr {
    fn check(&self, field: &Field<'_>) -> bool {
        field
            .value()
            .string()
            .is_some_and(|value| parse_cidr(value.as_ref()).is_some())
    }
}

#[derive(Debug)]
pub struct Cidrv4;

impl Rule for Cidrv4 {
    fn check(&self, field: &Field<'_>) -> bool {
        field
            .value()
            .string()
            .is_some_and(|value| matches!(parse_cidr(value.as_ref()), Some(CidrKind::V4)))
    }
}

#[derive(Debug)]
pub struct Cidrv6;

impl Rule for Cidrv6 {
    fn check(&self, field: &Field<'_>) -> bool {
        field
            .value()
            .string()
            .is_some_and(|value| matches!(parse_cidr(value.as_ref()), Some(CidrKind::V6)))
    }
}

enum CidrKind {
    V4,
    V6,
}

fn parse_cidr(value: &str) -> Option<CidrKind> {
    let (addr, prefix) = value.split_once('/')?;
    let prefix = prefix.parse::<u8>().ok()?;

    match addr.parse::<IpAddr>().ok()? {
        IpAddr::V4(_) if prefix <= 32 && addr.parse::<Ipv4Addr>().is_ok() => Some(CidrKind::V4),
        IpAddr::V6(_) if prefix <= 128 && addr.parse::<Ipv6Addr>().is_ok() => Some(CidrKind::V6),
        _ => None,
    }
}
