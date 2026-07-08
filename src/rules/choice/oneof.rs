use crate::{Field, Rule};

#[derive(Debug)]
pub struct OneOf;

impl Rule for OneOf {
    fn check(&self, field: &Field<'_>) -> Result<bool, crate::Error> {
        Ok(super::contains(field).unwrap_or(false))
    }
}
