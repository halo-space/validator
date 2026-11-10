use std::net::{IpAddr as StdIpAddr, Ipv4Addr, Ipv6Addr};

use crate::{Field, Rule};

#[derive(Debug)]
pub(crate) struct Ip;

#[async_trait::async_trait(?Send)]
impl Rule for Ip {
    async fn check(&self, field: &Field<'_>) -> Result<bool, crate::Error> {
        Ok(field
            .value()
            .string()
            .is_some_and(|value| value.parse::<StdIpAddr>().is_ok()))
    }
}

#[derive(Debug)]
pub(crate) struct Ipv4;

#[async_trait::async_trait(?Send)]
impl Rule for Ipv4 {
    async fn check(&self, field: &Field<'_>) -> Result<bool, crate::Error> {
        Ok(field
            .value()
            .string()
            .is_some_and(|value| value.parse::<Ipv4Addr>().is_ok()))
    }
}

#[derive(Debug)]
pub(crate) struct Ipv6;

#[async_trait::async_trait(?Send)]
impl Rule for Ipv6 {
    async fn check(&self, field: &Field<'_>) -> Result<bool, crate::Error> {
        Ok(field
            .value()
            .string()
            .is_some_and(|value| value.parse::<Ipv6Addr>().is_ok()))
    }
}
