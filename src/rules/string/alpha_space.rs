use crate::{Field, Rule};

#[derive(Debug)]
pub(crate) struct AlphaSpace;

#[async_trait::async_trait(?Send)]
impl Rule for AlphaSpace {
    async fn check(&self, field: &Field<'_>) -> Result<bool, crate::Error> {
        Ok(field.value().string().is_some_and(|value| {
            let value = value.as_ref();
            !value.is_empty()
                && value
                    .chars()
                    .all(|ch| ch.is_ascii_alphabetic() || ch == ' ')
        }))
    }
}
