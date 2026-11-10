use crate::{Field, Rule};

#[derive(Debug)]
pub(crate) struct Ascii;

#[async_trait::async_trait(?Send)]
impl Rule for Ascii {
    async fn check(&self, field: &Field<'_>) -> Result<bool, crate::Error> {
        Ok(field.value().string().is_some_and(|value| value.is_ascii()))
    }
}
