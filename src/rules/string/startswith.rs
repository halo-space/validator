use crate::{Field, Rule};

#[derive(Debug)]
pub struct StartsWith;

impl Rule for StartsWith {
    fn check(&self, field: &Field<'_>) -> bool {
        super::value_and_expected(field, "value")
            .is_some_and(|(value, expected)| value.starts_with(expected))
    }
}
