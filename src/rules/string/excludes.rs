use crate::{Field, Rule};

#[derive(Debug)]
pub struct Excludes;

impl Rule for Excludes {
    fn check(&self, field: &Field<'_>) -> Result<bool, crate::Error> {
        Ok(super::value_and_expected(field, "value")
            .is_some_and(|(value, expected)| !value.contains(expected)))
    }
}
