use crate::{Field, Rule, Signature};

#[derive(Debug)]
pub(crate) struct Contains;

#[async_trait::async_trait(?Send)]
impl Rule for Contains {
    fn signature(&self) -> Signature {
        Signature::text("value")
    }

    async fn check(&self, field: &Field<'_>) -> Result<bool, crate::Error> {
        Ok(super::value_and_expected(field, "value")
            .is_some_and(|(value, expected)| value.contains(expected)))
    }
}
