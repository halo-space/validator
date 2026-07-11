use std::borrow::Cow;

use serde_json::Value as JsonValue;

use super::{FloatKind, IntKind, Kind, UintKind, Value};

impl Value for JsonValue {
    fn kind(&self) -> Kind {
        match self {
            JsonValue::Null => Kind::Option,
            JsonValue::Bool(_) => Kind::Bool,
            JsonValue::Number(number) => {
                if number.as_i64().is_some() {
                    Kind::Int(IntKind::I64)
                } else if number.as_u64().is_some() {
                    Kind::Uint(UintKind::U64)
                } else {
                    Kind::Float(FloatKind::F64)
                }
            }
            JsonValue::String(_) => Kind::String,
            JsonValue::Array(_) => Kind::Vec,
            JsonValue::Object(_) => Kind::Map,
        }
    }

    fn is_none(&self) -> bool {
        self.is_null()
    }

    fn required(&self) -> bool {
        match self {
            JsonValue::Null => false,
            JsonValue::Bool(value) => *value,
            JsonValue::Number(number) => {
                number.as_i64().is_some_and(|value| value != 0)
                    || number.as_u64().is_some_and(|value| value != 0)
                    || number.as_f64().is_some_and(|value| value != 0.0)
            }
            JsonValue::String(value) => !value.is_empty(),
            JsonValue::Array(value) => !value.is_empty(),
            JsonValue::Object(value) => !value.is_empty(),
        }
    }

    fn string(&self) -> Option<Cow<'_, str>> {
        self.as_str().map(Cow::Borrowed)
    }

    fn len(&self) -> Option<usize> {
        match self {
            JsonValue::String(value) => Some(value.chars().count()),
            JsonValue::Array(value) => Some(value.len()),
            JsonValue::Object(value) => Some(value.len()),
            JsonValue::Null | JsonValue::Bool(_) | JsonValue::Number(_) => None,
        }
    }

    fn int(&self) -> Option<i128> {
        self.as_i64().map(i128::from)
    }

    fn uint(&self) -> Option<u128> {
        self.as_u64().map(u128::from)
    }

    fn float(&self) -> Option<f64> {
        self.as_f64()
    }

    fn boolean(&self) -> Option<bool> {
        self.as_bool()
    }

    fn array_items(&self) -> Option<Box<dyn Iterator<Item = &dyn Value> + '_>> {
        self.as_array().map(|items| {
            Box::new(items.iter().map(|item| item as &dyn Value))
                as Box<dyn Iterator<Item = &dyn Value>>
        })
    }

    fn map_values(&self) -> Option<Box<dyn Iterator<Item = &dyn Value> + '_>> {
        self.as_object().map(|object| {
            Box::new(object.values().map(|value| value as &dyn Value))
                as Box<dyn Iterator<Item = &dyn Value>>
        })
    }
}
