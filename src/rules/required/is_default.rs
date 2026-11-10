use crate::{Field, Rule};

#[derive(Debug)]
pub(crate) struct IsDefault;

impl Rule for IsDefault {
    async fn check(&self, field: &Field<'_>) -> Result<bool, crate::Error> {
        Ok(field.value().is_default())
    }
}
