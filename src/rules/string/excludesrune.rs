use crate::{Field, Rule};

#[derive(Debug)]
pub struct ExcludesRune;

impl Rule for ExcludesRune {
    fn check(&self, field: &Field<'_>) -> Result<bool, crate::Error> {
        let Some((value, expected)) = super::value_and_expected(field, "value") else {
            return Ok(false);
        };
        let Some(expected) = expected.chars().next() else {
            return Ok(false);
        };

        Ok(!value.contains(expected))
    }
}
