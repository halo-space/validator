use crate::{Field, Rule, Signature};

#[derive(Debug)]
pub struct Max;

impl Rule for Max {
    fn signature(&self) -> Signature {
        Signature::text("max")
    }

    fn validate_params(&self, field: &Field<'_>) -> Result<(), crate::Error> {
        super::validate(field, "max")
    }

    fn check(&self, field: &Field<'_>) -> Result<bool, crate::Error> {
        super::satisfies(field, "max", super::Relation::Lte)
    }
}
