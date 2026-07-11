use crate::{Field, Rule, Signature};

#[derive(Debug)]
pub(crate) struct Ne;

impl Rule for Ne {
    fn signature(&self) -> Signature {
        Signature::optional_text("value")
    }

    fn validate_params(&self, field: &Field<'_>) -> Result<(), crate::Error> {
        super::validate_equality(field, "ne")
    }

    fn check(&self, field: &Field<'_>) -> Result<bool, crate::Error> {
        super::not_equals(field)
    }
}
