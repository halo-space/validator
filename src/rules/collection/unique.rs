use crate::{Field, Rule};

#[derive(Debug)]
pub struct Unique;

impl Rule for Unique {
    fn check(&self, field: &Field<'_>) -> bool {
        field
            .value()
            .array_items()
            .or_else(|| field.value().map_values())
            .is_some_and(super::values_are_unique)
    }
}
