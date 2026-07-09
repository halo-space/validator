use crate::{Field, Rule};

#[derive(Debug)]
pub struct Multibyte;

impl Rule for Multibyte {
    fn check(&self, field: &Field<'_>) -> Result<bool, crate::Error> {
        Ok(field
            .value()
            .string()
            .is_some_and(|value| value.is_empty() || !value.is_ascii()))
    }
}
