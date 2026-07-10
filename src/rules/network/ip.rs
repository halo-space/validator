use std::net::{IpAddr as StdIpAddr, Ipv4Addr, Ipv6Addr};

use crate::{Field, Rule};

#[derive(Debug)]
pub struct Ip;

impl Rule for Ip {
    fn check(&self, field: &Field<'_>) -> Result<bool, crate::Error> {
        Ok(field
            .value()
            .string()
            .is_some_and(|value| value.parse::<StdIpAddr>().is_ok()))
    }
}

#[derive(Debug)]
pub struct Ipv4;

impl Rule for Ipv4 {
    fn check(&self, field: &Field<'_>) -> Result<bool, crate::Error> {
        Ok(field
            .value()
            .string()
            .is_some_and(|value| value.parse::<Ipv4Addr>().is_ok()))
    }
}

#[derive(Debug)]
pub struct Ipv6;

impl Rule for Ipv6 {
    fn check(&self, field: &Field<'_>) -> Result<bool, crate::Error> {
        Ok(field
            .value()
            .string()
            .is_some_and(|value| value.parse::<Ipv6Addr>().is_ok()))
    }
}
