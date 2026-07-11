use std::borrow::Cow;
use std::time::SystemTime;

mod collection;
mod json;
mod network;
mod primitive;

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

    /// Returns the type's stable Kind, or `None` when Kind depends on the value.
    fn declared_kind() -> Option<Kind>
    where
        Self: Sized,
    {
        None
    }

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

    fn time(&self) -> Option<SystemTime> {
        None
    }

    fn number(&self) -> Option<Number> {
        self.int()
            .map(Number::Int)
            .or_else(|| self.uint().map(Number::Uint))
            .or_else(|| self.float().map(Number::Float))
    }

    fn array_items(&self) -> Option<Box<dyn Iterator<Item = &dyn Value> + '_>> {
        None
    }

    fn map_values(&self) -> Option<Box<dyn Iterator<Item = &dyn Value> + '_>> {
        None
    }
}
