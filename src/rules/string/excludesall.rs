use crate::{Field, Rule};

#[derive(Debug)]
pub struct ExcludesAll;

impl Rule for ExcludesAll {
    fn check(&self, field: &Field<'_>) -> Result<bool, crate::Error> {
        Ok(super::value_and_expected(field, "value")
            .is_some_and(|(value, expected)| !value.chars().any(|ch| expected.contains(ch))))
    }
}
