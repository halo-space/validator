use std::borrow::Cow;
use std::time::SystemTime;

mod collection;
mod json;
mod network;
mod primitive;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
/// Runtime and declared value families used for rule dispatch.
pub enum Kind {
    /// UTF-8 text.
    String,
    /// A boolean value.
    Bool,
    /// A signed integer of a specific Rust width.
    Int(IntKind),
    /// An unsigned integer of a specific Rust width.
    Uint(UintKind),
    /// A floating-point value of a specific Rust width.
    Float(FloatKind),
    /// A vector.
    Vec,
    /// A fixed-size array.
    Array,
    /// A slice.
    Slice,
    /// A map.
    Map,
    /// An absent optional value.
    Option,
    /// A [`SystemTime`] value.
    Time,
    /// A value without a more specific built-in family.
    Other,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
/// Signed integer widths.
pub enum IntKind {
    /// `i8`.
    I8,
    /// `i16`.
    I16,
    /// `i32`.
    I32,
    /// `i64`.
    I64,
    /// `i128`.
    I128,
    /// `isize`.
    Isize,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
/// Unsigned integer widths.
pub enum UintKind {
    /// `u8`.
    U8,
    /// `u16`.
    U16,
    /// `u32`.
    U32,
    /// `u64`.
    U64,
    /// `u128`.
    U128,
    /// `usize`.
    Usize,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
/// Floating-point widths.
pub enum FloatKind {
    /// `f32`.
    F32,
    /// `f64`.
    F64,
}

#[derive(Clone, Copy, Debug, PartialEq)]
/// A normalized numeric value exposed by [`Value::number`].
pub enum Number {
    /// A signed integer.
    Int(i128),
    /// An unsigned integer.
    Uint(u128),
    /// A floating-point number.
    Float(f64),
}

/// Adapts a Rust type to validator's typed rule-dispatch model.
pub trait Value {
    /// Returns the current value's kind.
    fn kind(&self) -> Kind;

    /// Returns the type's stable Kind, or `None` when Kind depends on the value.
    fn declared_kind() -> Option<Kind>
    where
        Self: Sized,
    {
        None
    }

    /// Returns whether this value represents an absent optional value.
    fn is_none(&self) -> bool {
        false
    }

    /// Returns whether this value satisfies the `required` rule.
    fn required(&self) -> bool;

    /// Exposes this value as text when supported.
    fn string(&self) -> Option<Cow<'_, str>> {
        None
    }

    /// Returns character or collection length when supported.
    fn len(&self) -> Option<usize> {
        None
    }

    /// Returns whether a length-bearing value is empty.
    fn is_empty(&self) -> Option<bool> {
        self.len().map(|len| len == 0)
    }

    /// Exposes this value as a signed integer.
    fn int(&self) -> Option<i128> {
        None
    }

    /// Exposes this value as an unsigned integer.
    fn uint(&self) -> Option<u128> {
        None
    }

    /// Exposes this value as a floating-point number.
    fn float(&self) -> Option<f64> {
        None
    }

    /// Exposes this value as a boolean.
    fn boolean(&self) -> Option<bool> {
        None
    }

    /// Exposes this value as a system time.
    fn time(&self) -> Option<SystemTime> {
        None
    }

    /// Exposes this value through a normalized numeric representation.
    fn number(&self) -> Option<Number> {
        self.int()
            .map(Number::Int)
            .or_else(|| self.uint().map(Number::Uint))
            .or_else(|| self.float().map(Number::Float))
    }

    /// Iterates array-like elements for collection rules.
    fn array_items(&self) -> Option<Box<dyn Iterator<Item = &dyn Value> + '_>> {
        None
    }

    /// Iterates map values for collection rules.
    fn map_values(&self) -> Option<Box<dyn Iterator<Item = &dyn Value> + '_>> {
        None
    }
}
