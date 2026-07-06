use crate::{Field, Rule};

#[derive(Debug)]
pub struct EndsWith;

impl Rule for EndsWith {
    fn check(&self, field: &Field<'_>) -> bool {
        super::value_and_expected(field, "value")
            .is_some_and(|(value, expected)| value.ends_with(expected))
    }
}
