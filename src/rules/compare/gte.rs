use crate::{Field, Rule, Signature};

#[derive(Debug)]
pub(crate) struct Gte;

#[async_trait::async_trait(?Send)]
impl Rule for Gte {
    fn signature(&self) -> Signature {
        Signature::optional_text("value")
    }

    fn validate_params(&self, field: &Field<'_>) -> Result<(), crate::Error> {
        super::validate_satisfies(field, "value")
    }

    async fn check(&self, field: &Field<'_>) -> Result<bool, crate::Error> {
        super::satisfies(field, "value", super::Relation::Gte)
    }
}
