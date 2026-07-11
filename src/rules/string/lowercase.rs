use crate::{Field, Rule};

#[derive(Debug)]
pub(crate) struct Lowercase;

impl Rule for Lowercase {
    fn check(&self, field: &Field<'_>) -> Result<bool, crate::Error> {
        Ok(field.value().string().is_some_and(|value| {
            let value = value.as_ref();
            !value.is_empty() && value == value.to_lowercase()
        }))
    }
}
