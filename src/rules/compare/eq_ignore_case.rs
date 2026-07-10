use crate::{Field, Rule, Signature};

#[derive(Debug)]
pub struct EqIgnoreCase;

impl Rule for EqIgnoreCase {
    fn signature(&self) -> Signature {
        Signature::text("value")
    }

    fn check(&self, field: &Field<'_>) -> Result<bool, crate::Error> {
        Ok(compare(field).unwrap_or(false))
    }
}

pub(super) fn compare(field: &Field<'_>) -> Option<bool> {
    let value = field.value().string()?;
    let expected = field.params().text("value")?;

    Some(value.to_lowercase() == expected.to_lowercase())
}
