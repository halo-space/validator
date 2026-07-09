use std::net::SocketAddr;

use crate::{Field, Rule};

#[derive(Debug)]
pub struct Tcp4Address;

#[derive(Debug)]
pub struct Tcp6Address;

#[derive(Debug)]
pub struct TcpAddress;

#[derive(Debug)]
pub struct Udp4Address;

#[derive(Debug)]
pub struct Udp6Address;

#[derive(Debug)]
pub struct UdpAddress;

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

socket_rule!(Tcp4Address, SocketAddr::is_ipv4);
socket_rule!(Tcp6Address, SocketAddr::is_ipv6);
socket_rule!(TcpAddress, |_| true);
socket_rule!(Udp4Address, SocketAddr::is_ipv4);
socket_rule!(Udp6Address, SocketAddr::is_ipv6);
socket_rule!(UdpAddress, |_| true);

fn valid(value: &str, check: impl Fn(&SocketAddr) -> bool) -> bool {
    value
        .parse::<SocketAddr>()
        .is_ok_and(|address| check(&address))
}
