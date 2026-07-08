use crate::{Field, Rule};

#[derive(Debug)]
pub struct Ascii;

impl Rule for Ascii {
    fn check(&self, field: &Field<'_>) -> Result<bool, crate::Error> {
        Ok(field.value().string().is_some_and(|value| value.is_ascii()))
    }
}
