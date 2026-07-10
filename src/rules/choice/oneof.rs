use crate::{Field, Rule, Signature};

#[derive(Debug)]
pub struct OneOf;

impl Rule for OneOf {
    fn signature(&self) -> Signature {
        Signature::list("values")
    }

    fn check(&self, field: &Field<'_>) -> Result<bool, crate::Error> {
        Ok(super::contains(field).unwrap_or(false))
    }
}
