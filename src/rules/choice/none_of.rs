use crate::{Field, Rule, Signature};

#[derive(Debug)]
pub(crate) struct NoneOf;

#[async_trait::async_trait(?Send)]
impl Rule for NoneOf {
    fn signature(&self) -> Signature {
        Signature::list("values")
    }

    fn validate_params(&self, field: &Field<'_>) -> Result<(), crate::Error> {
        super::validate(field, "noneof")
    }

    async fn check(&self, field: &Field<'_>) -> Result<bool, crate::Error> {
        Ok(super::contains(field, "noneof")?.is_some_and(|contains| !contains))
    }
}
