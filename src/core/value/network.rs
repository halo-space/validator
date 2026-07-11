use std::borrow::Cow;
use std::net::{IpAddr, Ipv4Addr, Ipv6Addr};

use super::{Kind, Value};

impl Value for IpAddr {
    fn kind(&self) -> Kind {
        Kind::Other
    }

    fn declared_kind() -> Option<Kind> {
        Some(Kind::Other)
    }

    fn required(&self) -> bool {
        true
    }

    fn string(&self) -> Option<Cow<'_, str>> {
        Some(Cow::Owned(self.to_string()))
    }
}

impl Value for Ipv4Addr {
    fn kind(&self) -> Kind {
        Kind::Other
    }

    fn declared_kind() -> Option<Kind> {
        Some(Kind::Other)
    }

    fn required(&self) -> bool {
        true
    }

    fn string(&self) -> Option<Cow<'_, str>> {
        Some(Cow::Owned(self.to_string()))
    }
}

impl Value for Ipv6Addr {
    fn kind(&self) -> Kind {
        Kind::Other
    }

    fn declared_kind() -> Option<Kind> {
        Some(Kind::Other)
    }

    fn required(&self) -> bool {
        true
    }

    fn string(&self) -> Option<Cow<'_, str>> {
        Some(Cow::Owned(self.to_string()))
    }
}
