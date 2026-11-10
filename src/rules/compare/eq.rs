use crate::{Field, Rule, Signature};

#[derive(Debug)]
pub(crate) struct Eq;

#[async_trait::async_trait(?Send)]
impl Rule for Eq {
    fn signature(&self) -> Signature {
        Signature::optional_text("value")
    }

    fn validate_params(&self, field: &Field<'_>) -> Result<(), crate::Error> {
        super::validate_equality(field, "eq")
    }

    async fn check(&self, field: &Field<'_>) -> Result<bool, crate::Error> {
        super::equals(field)
    }
}
