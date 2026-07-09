use crate::{Field, Rule};

#[derive(Debug)]
pub struct OneOfCi;

impl Rule for OneOfCi {
    fn check(&self, field: &Field<'_>) -> Result<bool, crate::Error> {
        Ok(super::contains_ignore_case(field).unwrap_or(false))
    }
}
