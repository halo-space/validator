use crate::{Field, Rule, Signature};

#[derive(Debug)]
pub(crate) struct EndsWith;

#[async_trait::async_trait(?Send)]
impl Rule for EndsWith {
    fn signature(&self) -> Signature {
        Signature::text("value")
    }

    async fn check(&self, field: &Field<'_>) -> Result<bool, crate::Error> {
        Ok(super::value_and_expected(field, "value")
            .is_some_and(|(value, expected)| value.ends_with(expected)))
    }
}
