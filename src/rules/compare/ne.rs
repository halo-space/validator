use crate::{Field, Rule, Signature};

#[derive(Debug)]
pub struct Ne;

impl Rule for Ne {
    fn signature(&self) -> Signature {
        Signature::optional_text("value")
    }

    fn check(&self, field: &Field<'_>) -> Result<bool, crate::Error> {
        super::not_equals(field)
    }
}
