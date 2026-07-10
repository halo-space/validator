use crate::{Field, Rule, Signature};

#[derive(Debug)]
pub struct NoneOf;

impl Rule for NoneOf {
    fn signature(&self) -> Signature {
        Signature::list("values")
    }

    fn check(&self, field: &Field<'_>) -> Result<bool, crate::Error> {
        Ok(super::contains(field).is_some_and(|contains| !contains))
    }
}
