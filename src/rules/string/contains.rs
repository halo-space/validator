use crate::{Field, Rule};

#[derive(Debug)]
pub struct Contains;

impl Rule for Contains {
    fn check(&self, field: &Field<'_>) -> bool {
        super::value_and_expected(field, "value")
            .is_some_and(|(value, expected)| value.contains(expected))
    }
}
