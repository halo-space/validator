use crate::{Field, Rule};

#[derive(Debug)]
pub struct ContainsAny;

impl Rule for ContainsAny {
    fn check(&self, field: &Field<'_>) -> bool {
        super::value_and_expected(field, "value")
            .is_some_and(|(value, expected)| value.chars().any(|ch| expected.contains(ch)))
    }
}
