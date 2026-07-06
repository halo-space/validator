use crate::{Field, Rule};

#[derive(Debug)]
pub struct Boolean;

impl Rule for Boolean {
    fn check(&self, field: &Field<'_>) -> bool {
        if field.value().boolean().is_some() {
            return true;
        }

        field
            .value()
            .string()
            .is_some_and(|value| parse_bool(value.as_ref()))
    }
}

fn parse_bool(value: &str) -> bool {
    matches!(
        value,
        "1" | "t" | "T" | "true" | "TRUE" | "True" | "0" | "f" | "F" | "false" | "FALSE" | "False"
    )
}
