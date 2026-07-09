use crate::{Field, Rule};

#[derive(Debug)]
pub struct StartsNotWith;

impl Rule for StartsNotWith {
    fn check(&self, field: &Field<'_>) -> Result<bool, crate::Error> {
        Ok(super::value_and_expected(field, "value")
            .is_some_and(|(value, expected)| !value.starts_with(expected)))
    }
}
