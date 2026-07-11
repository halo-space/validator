use crate::{Field, Rule, Signature};

#[derive(Debug)]
pub(crate) struct Min;

impl Rule for Min {
    fn signature(&self) -> Signature {
        Signature::text("min")
    }

    fn validate_params(&self, field: &Field<'_>) -> Result<(), crate::Error> {
        super::validate(field, "min")
    }

    fn check(&self, field: &Field<'_>) -> Result<bool, crate::Error> {
        super::satisfies(field, "min", super::Relation::Gte)
    }
}
