use crate::{Field, Rule};

#[derive(Debug)]
pub struct Contains;

impl Rule for Contains {
    fn check(&self, field: &Field<'_>) -> Result<bool, crate::Error> {
        Ok(super::value_and_expected(field, "value")
            .is_some_and(|(value, expected)| value.contains(expected)))
    }
}
