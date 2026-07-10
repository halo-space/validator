use crate::{Field, Rule, Signature};

#[derive(Debug)]
pub struct NoneOf;

impl Rule for NoneOf {
    fn signature(&self) -> Signature {
        Signature::list("values")
    }

    fn validate_params(&self, field: &Field<'_>) -> Result<(), crate::Error> {
        super::validate(field, "noneof")
    }

    fn check(&self, field: &Field<'_>) -> Result<bool, crate::Error> {
        Ok(super::contains(field, "noneof")?.is_some_and(|contains| !contains))
    }
}
