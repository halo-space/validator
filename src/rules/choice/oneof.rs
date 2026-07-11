use crate::{Field, Rule, Signature};

#[derive(Debug)]
pub(crate) struct OneOf;

impl Rule for OneOf {
    fn signature(&self) -> Signature {
        Signature::list("values")
    }

    fn validate_params(&self, field: &Field<'_>) -> Result<(), crate::Error> {
        super::validate(field, "oneof")
    }

    fn check(&self, field: &Field<'_>) -> Result<bool, crate::Error> {
        Ok(super::contains(field, "oneof")?.unwrap_or(false))
    }
}
