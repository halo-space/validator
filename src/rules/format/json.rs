use serde_json::Value as JsonValue;

use crate::{Field, Rule};

#[derive(Debug)]
pub struct Json;

impl Rule for Json {
    fn check(&self, field: &Field<'_>) -> Result<bool, crate::Error> {
        Ok(field
            .value()
            .string()
            .is_some_and(|value| serde_json::from_str::<JsonValue>(value.as_ref()).is_ok()))
    }
}
