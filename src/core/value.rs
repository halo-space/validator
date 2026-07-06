use std::borrow::Cow;
use std::collections::{BTreeMap, HashMap};
use std::hash::Hash;
use std::net::{IpAddr, Ipv4Addr, Ipv6Addr};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Kind {
    String,
    Bool,
    Int(IntKind),
    Uint(UintKind),
    Float(FloatKind),
    Vec,
    Array,
    Slice,
    Map,
    Option,
    Time,
    Other,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum IntKind {
    I8,
    I16,
    I32,
    I64,
    I128,
    Isize,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum UintKind {
    U8,
    U16,
    U32,
    U64,
    U128,
    Usize,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum FloatKind {
    F32,
    F64,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Number {
    Int(i128),
    Uint(u128),
    Float(f64),
}

pub trait Value {
    fn kind(&self) -> Kind;

    fn is_none(&self) -> bool {
        false
    }

    fn required(&self) -> bool;

    fn string(&self) -> Option<Cow<'_, str>> {
        None
    }

    fn len(&self) -> Option<usize> {
        None
    }

    fn is_empty(&self) -> Option<bool> {
        self.len().map(|len| len == 0)
    }

    fn int(&self) -> Option<i128> {
        None
    }

    fn uint(&self) -> Option<u128> {
        None
    }

    fn float(&self) -> Option<f64> {
        None
    }

    fn boolean(&self) -> Option<bool> {
        None
    }

    fn number(&self) -> Option<Number> {
        self.int()
            .map(Number::Int)
            .or_else(|| self.uint().map(Number::Uint))
            .or_else(|| self.float().map(Number::Float))
    }
}

impl<T: Value> Value for Option<T> {
    fn kind(&self) -> Kind {
        self.as_ref().map_or(Kind::Option, Value::kind)
    }

    fn is_none(&self) -> bool {
        self.is_none()
    }

    fn required(&self) -> bool {
        self.is_some()
    }

    fn string(&self) -> Option<Cow<'_, str>> {
        self.as_ref().and_then(Value::string)
    }

    fn len(&self) -> Option<usize> {
        self.as_ref().and_then(Value::len)
    }

    fn int(&self) -> Option<i128> {
        self.as_ref().and_then(Value::int)
    }

    fn uint(&self) -> Option<u128> {
        self.as_ref().and_then(Value::uint)
    }

    fn float(&self) -> Option<f64> {
        self.as_ref().and_then(Value::float)
    }

    fn boolean(&self) -> Option<bool> {
        self.as_ref().and_then(Value::boolean)
    }
}

impl Value for String {
    fn kind(&self) -> Kind {
        Kind::String
    }

    fn required(&self) -> bool {
        !self.as_str().is_empty()
    }

    fn string(&self) -> Option<Cow<'_, str>> {
        Some(Cow::Borrowed(self.as_str()))
    }

    fn len(&self) -> Option<usize> {
        Some(self.chars().count())
    }
}

impl Value for str {
    fn kind(&self) -> Kind {
        Kind::String
    }

    fn required(&self) -> bool {
        !str::is_empty(self)
    }

    fn string(&self) -> Option<Cow<'_, str>> {
        Some(Cow::Borrowed(self))
    }

    fn len(&self) -> Option<usize> {
        Some(self.chars().count())
    }
}

impl Value for &str {
    fn kind(&self) -> Kind {
        Kind::String
    }

    fn required(&self) -> bool {
        !str::is_empty(self)
    }

    fn string(&self) -> Option<Cow<'_, str>> {
        Some(Cow::Borrowed(*self))
    }

    fn len(&self) -> Option<usize> {
        Some(self.chars().count())
    }
}

impl Value for bool {
    fn kind(&self) -> Kind {
        Kind::Bool
    }

    fn required(&self) -> bool {
        *self
    }

    fn boolean(&self) -> Option<bool> {
        Some(*self)
    }
}

macro_rules! impl_signed_value {
    ($($ty:ty => $kind:ident),* $(,)?) => {
        $(
            impl Value for $ty {
                fn kind(&self) -> Kind {
                    Kind::Int(IntKind::$kind)
                }

                fn required(&self) -> bool {
                    *self != 0
                }

                fn int(&self) -> Option<i128> {
                    Some(*self as i128)
                }
            }
        )*
    };
}

macro_rules! impl_unsigned_value {
    ($($ty:ty => $kind:ident),* $(,)?) => {
        $(
            impl Value for $ty {
                fn kind(&self) -> Kind {
                    Kind::Uint(UintKind::$kind)
                }

                fn required(&self) -> bool {
                    *self != 0
                }

                fn uint(&self) -> Option<u128> {
                    Some(*self as u128)
                }
            }
        )*
    };
}

impl_signed_value!(
    i8 => I8,
    i16 => I16,
    i32 => I32,
    i64 => I64,
    i128 => I128,
    isize => Isize,
);

impl_unsigned_value!(
    u8 => U8,
    u16 => U16,
    u32 => U32,
    u64 => U64,
    u128 => U128,
    usize => Usize,
);

impl Value for f32 {
    fn kind(&self) -> Kind {
        Kind::Float(FloatKind::F32)
    }

    fn required(&self) -> bool {
        *self != 0.0
    }

    fn float(&self) -> Option<f64> {
        Some(*self as f64)
    }
}

impl Value for f64 {
    fn kind(&self) -> Kind {
        Kind::Float(FloatKind::F64)
    }

    fn required(&self) -> bool {
        *self != 0.0
    }

    fn float(&self) -> Option<f64> {
        Some(*self)
    }
}

impl<T> Value for Vec<T> {
    fn kind(&self) -> Kind {
        Kind::Vec
    }

    fn required(&self) -> bool {
        !self.is_empty()
    }

    fn len(&self) -> Option<usize> {
        Some(self.len())
    }
}

impl<T, const N: usize> Value for [T; N] {
    fn kind(&self) -> Kind {
        Kind::Array
    }

    fn required(&self) -> bool {
        N > 0
    }

    fn len(&self) -> Option<usize> {
        Some(N)
    }
}

impl<T> Value for [T] {
    fn kind(&self) -> Kind {
        Kind::Slice
    }

    fn required(&self) -> bool {
        !self.is_empty()
    }

    fn len(&self) -> Option<usize> {
        Some(self.len())
    }
}

impl<K, V> Value for BTreeMap<K, V> {
    fn kind(&self) -> Kind {
        Kind::Map
    }

    fn required(&self) -> bool {
        !self.is_empty()
    }

    fn len(&self) -> Option<usize> {
        Some(self.len())
    }
}

impl<K: Eq + Hash, V> Value for HashMap<K, V> {
    fn kind(&self) -> Kind {
        Kind::Map
    }

    fn required(&self) -> bool {
        !self.is_empty()
    }

    fn len(&self) -> Option<usize> {
        Some(self.len())
    }
}

impl Value for IpAddr {
    fn kind(&self) -> Kind {
        Kind::Other
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

    fn required(&self) -> bool {
        true
    }

    fn string(&self) -> Option<Cow<'_, str>> {
        Some(Cow::Owned(self.to_string()))
    }
}
