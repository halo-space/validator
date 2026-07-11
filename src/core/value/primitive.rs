use std::borrow::Cow;
use std::time::SystemTime;

use super::{FloatKind, IntKind, Kind, UintKind, Value};

impl<T: Value> Value for Option<T> {
    fn kind(&self) -> Kind {
        self.as_ref().map_or(Kind::Option, Value::kind)
    }

    fn declared_kind() -> Option<Kind> {
        T::declared_kind()
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

    fn time(&self) -> Option<SystemTime> {
        self.as_ref().and_then(Value::time)
    }

    fn array_items(&self) -> Option<Box<dyn Iterator<Item = &dyn Value> + '_>> {
        self.as_ref().and_then(Value::array_items)
    }

    fn map_values(&self) -> Option<Box<dyn Iterator<Item = &dyn Value> + '_>> {
        self.as_ref().and_then(Value::map_values)
    }
}

impl Value for String {
    fn kind(&self) -> Kind {
        Kind::String
    }

    fn declared_kind() -> Option<Kind> {
        Some(Kind::String)
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

    fn declared_kind() -> Option<Kind> {
        Some(Kind::String)
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

    fn declared_kind() -> Option<Kind> {
        Some(Kind::Bool)
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

                fn declared_kind() -> Option<Kind> {
                    Some(Kind::Int(IntKind::$kind))
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

                fn declared_kind() -> Option<Kind> {
                    Some(Kind::Uint(UintKind::$kind))
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

    fn declared_kind() -> Option<Kind> {
        Some(Kind::Float(FloatKind::F32))
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

    fn declared_kind() -> Option<Kind> {
        Some(Kind::Float(FloatKind::F64))
    }

    fn required(&self) -> bool {
        *self != 0.0
    }

    fn float(&self) -> Option<f64> {
        Some(*self)
    }
}

impl Value for SystemTime {
    fn kind(&self) -> Kind {
        Kind::Time
    }

    fn declared_kind() -> Option<Kind> {
        Some(Kind::Time)
    }

    fn required(&self) -> bool {
        true
    }

    fn time(&self) -> Option<SystemTime> {
        Some(*self)
    }
}
