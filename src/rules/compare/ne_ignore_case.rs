use crate::{Field, Rule, Signature};

#[derive(Debug)]
pub(crate) struct NeIgnoreCase;

impl Rule for NeIgnoreCase {
    fn signature(&self) -> Signature {
        Signature::text("value")
    }

    fn check(&self, field: &Field<'_>) -> Result<bool, crate::Error> {
        Ok(super::eq_ignore_case::compare(field).is_some_and(|equal| !equal))
    }
}
