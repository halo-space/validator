use crate::{Field, Rule};

#[derive(Debug)]
pub struct NoneOfCi;

impl Rule for NoneOfCi {
    fn check(&self, field: &Field<'_>) -> Result<bool, crate::Error> {
        Ok(super::contains_ignore_case(field).is_some_and(|contains| !contains))
    }
}
