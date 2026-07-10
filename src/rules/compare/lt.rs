use crate::{Field, Rule, Signature};

#[derive(Debug)]
pub struct Lt;

impl Rule for Lt {
    fn signature(&self) -> Signature {
        Signature::optional_text("value")
    }

    fn validate_params(&self, field: &Field<'_>) -> Result<(), crate::Error> {
        super::validate_satisfies(field, "value")
    }

    fn check(&self, field: &Field<'_>) -> Result<bool, crate::Error> {
        super::satisfies(field, "value", super::Relation::Lt)
    }
}
