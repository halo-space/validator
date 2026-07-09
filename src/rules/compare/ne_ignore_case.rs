use crate::{Field, Rule};

#[derive(Debug)]
pub struct NeIgnoreCase;

impl Rule for NeIgnoreCase {
    fn check(&self, field: &Field<'_>) -> Result<bool, crate::Error> {
        Ok(super::eq_ignore_case::compare(field).is_some_and(|equal| !equal))
    }
}
