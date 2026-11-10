use crate::{Field, Rule, Signature};

#[derive(Debug)]
pub(crate) struct ExcludesRune;

#[async_trait::async_trait(?Send)]
impl Rule for ExcludesRune {
    fn signature(&self) -> Signature {
        Signature::text("value")
    }

    async fn check(&self, field: &Field<'_>) -> Result<bool, crate::Error> {
        let Some((value, expected)) = super::value_and_expected(field, "value") else {
            return Ok(false);
        };
        let Some(expected) = expected.chars().next() else {
            return Ok(false);
        };

        Ok(!value.contains(expected))
    }
}
