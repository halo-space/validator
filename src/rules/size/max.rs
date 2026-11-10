use crate::{Field, Rule, Signature};

#[derive(Debug)]
pub(crate) struct Max;

#[async_trait::async_trait(?Send)]
impl Rule for Max {
    fn signature(&self) -> Signature {
        Signature::text("max")
    }

    fn validate_params(&self, field: &Field<'_>) -> Result<(), crate::Error> {
        super::validate(field, "max")
    }

    async fn check(&self, field: &Field<'_>) -> Result<bool, crate::Error> {
        super::satisfies(field, "max", super::Relation::Lte)
    }
}
