use crate::{Field, Rule};

#[derive(Debug)]
pub(crate) struct Lowercase;

#[async_trait::async_trait(?Send)]
impl Rule for Lowercase {
    async fn check(&self, field: &Field<'_>) -> Result<bool, crate::Error> {
        Ok(field.value().string().is_some_and(|value| {
            let value = value.as_ref();
            !value.is_empty() && value == value.to_lowercase()
        }))
    }
}
