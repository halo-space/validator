use std::net::SocketAddr;

use crate::{Field, Rule};

#[derive(Debug)]
pub struct Tcp4;

#[derive(Debug)]
pub struct Tcp6;

#[derive(Debug)]
pub struct Tcp;

#[derive(Debug)]
pub struct Udp4;

#[derive(Debug)]
pub struct Udp6;

#[derive(Debug)]
pub struct Udp;

macro_rules! socket_rule {
    ($ty:ty, $check:expr) => {
        impl Rule for $ty {
            fn check(&self, field: &Field<'_>) -> Result<bool, crate::Error> {
                Ok(field
                    .value()
                    .string()
                    .is_some_and(|value| valid(value.as_ref(), $check)))
            }
        }
    };
}

socket_rule!(Tcp4, SocketAddr::is_ipv4);
socket_rule!(Tcp6, SocketAddr::is_ipv6);
socket_rule!(Tcp, |_| true);
socket_rule!(Udp4, SocketAddr::is_ipv4);
socket_rule!(Udp6, SocketAddr::is_ipv6);
socket_rule!(Udp, |_| true);

fn valid(value: &str, check: impl Fn(&SocketAddr) -> bool) -> bool {
    value
        .parse::<SocketAddr>()
        .is_ok_and(|address| check(&address))
}
