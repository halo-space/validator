use crate::{Field, Rule};

#[derive(Debug)]
pub struct EqIgnoreCase;

impl Rule for EqIgnoreCase {
    fn check(&self, field: &Field<'_>) -> Result<bool, crate::Error> {
        Ok(compare(field).unwrap_or(false))
    }
}

pub(super) fn compare(field: &Field<'_>) -> Option<bool> {
    let value = field.value().string()?;
    let expected = field.params().get("value")?;

    Some(value.to_lowercase() == expected.to_lowercase())
}
