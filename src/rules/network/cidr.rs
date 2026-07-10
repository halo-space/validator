use std::net::IpAddr;

use crate::{Field, Rule};

#[derive(Debug)]
pub struct Cidr;

impl Rule for Cidr {
    fn check(&self, field: &Field<'_>) -> Result<bool, crate::Error> {
        Ok(field
            .value()
            .string()
            .is_some_and(|value| parse(value.as_ref()).is_some()))
    }
}

#[derive(Debug)]
pub struct Cidrv4;

impl Rule for Cidrv4 {
    fn check(&self, field: &Field<'_>) -> Result<bool, crate::Error> {
        Ok(field
            .value()
            .string()
            .is_some_and(|value| matches!(parse(value.as_ref()), Some(Kind::V4 { network: true }))))
    }
}

#[derive(Debug)]
pub struct Cidrv6;

impl Rule for Cidrv6 {
    fn check(&self, field: &Field<'_>) -> Result<bool, crate::Error> {
        Ok(field
            .value()
            .string()
            .is_some_and(|value| matches!(parse(value.as_ref()), Some(Kind::V6))))
    }
}

enum Kind {
    V4 { network: bool },
    V6,
}

fn parse(value: &str) -> Option<Kind> {
    let (addr, prefix) = value.split_once('/')?;
    let prefix = prefix.parse::<u8>().ok()?;

    match addr.parse::<IpAddr>().ok()? {
        IpAddr::V4(addr) if prefix <= 32 => {
            let mask = if prefix == 0 {
                0
            } else {
                u32::MAX << (32 - prefix)
            };
            let addr = u32::from(addr);
            Some(Kind::V4 {
                network: addr & mask == addr,
            })
        }
        IpAddr::V6(_) if prefix <= 128 => Some(Kind::V6),
        _ => None,
    }
}
