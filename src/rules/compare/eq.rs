use crate::{Field, Rule, Signature};

#[derive(Debug)]
pub struct Eq;

impl Rule for Eq {
    fn signature(&self) -> Signature {
        Signature::optional_text("value")
    }

    fn check(&self, field: &Field<'_>) -> Result<bool, crate::Error> {
        super::equals(field)
    }
}
