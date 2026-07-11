use std::borrow::Cow;

use serde_json::Value as JsonValue;

use crate::core::{Kind, Value as CoreValue};

use super::Type;

pub(super) struct Field<'a> {
    value: Option<&'a JsonValue>,
    ty: Option<Type>,
}

impl<'a> Field<'a> {
    pub(super) fn new(value: Option<&'a JsonValue>, ty: Option<Type>) -> Self {
        Self { value, ty }
    }

    pub(super) fn raw(&self) -> Option<&'a JsonValue> {
        self.value
    }
}

impl CoreValue for Field<'_> {
    fn kind(&self) -> Kind {
        self.ty
            .map(Type::kind)
            .or_else(|| self.value.map(CoreValue::kind))
            .unwrap_or(Kind::Option)
    }

    fn is_none(&self) -> bool {
        self.value.is_none_or(JsonValue::is_null)
    }

    fn required(&self) -> bool {
        self.value.is_some_and(CoreValue::required)
    }

    fn string(&self) -> Option<Cow<'_, str>> {
        self.value.and_then(CoreValue::string)
    }

    fn len(&self) -> Option<usize> {
        self.value.and_then(CoreValue::len)
    }

    fn int(&self) -> Option<i128> {
        self.value.and_then(CoreValue::int)
    }

    fn uint(&self) -> Option<u128> {
        self.value.and_then(CoreValue::uint)
    }

    fn float(&self) -> Option<f64> {
        self.value.and_then(CoreValue::float)
    }

    fn boolean(&self) -> Option<bool> {
        self.value.and_then(CoreValue::boolean)
    }

    fn array_items(&self) -> Option<Box<dyn Iterator<Item = &dyn CoreValue> + '_>> {
        self.value.and_then(CoreValue::array_items)
    }

    fn map_values(&self) -> Option<Box<dyn Iterator<Item = &dyn CoreValue> + '_>> {
        self.value.and_then(CoreValue::map_values)
    }
}
