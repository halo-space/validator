use crate::{Field, Rule, Signature};

#[derive(Debug)]
pub(crate) struct NoneOfCi;

impl Rule for NoneOfCi {
    fn signature(&self) -> Signature {
        Signature::list("values")
    }

    fn check(&self, field: &Field<'_>) -> Result<bool, crate::Error> {
        Ok(super::contains_ignore_case(field).is_some_and(|contains| !contains))
    }
}
