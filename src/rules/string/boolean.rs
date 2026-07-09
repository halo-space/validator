use crate::{Field, Rule};

#[derive(Debug)]
pub struct Boolean;

impl Rule for Boolean {
    fn check(&self, field: &Field<'_>) -> Result<bool, crate::Error> {
        if field.value().boolean().is_some() {
            return Ok(true);
        }

        Ok(field
            .value()
            .string()
            .is_some_and(|value| parse(value.as_ref())))
    }
}

fn parse(value: &str) -> bool {
    matches!(
        value,
        "1" | "t" | "T" | "true" | "TRUE" | "True" | "0" | "f" | "F" | "false" | "FALSE" | "False"
    )
}
