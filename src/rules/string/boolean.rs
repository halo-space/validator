use crate::{Field, Rule};

#[derive(Debug)]
pub(crate) struct Boolean;

#[async_trait::async_trait(?Send)]
impl Rule for Boolean {
    async fn check(&self, field: &Field<'_>) -> Result<bool, crate::Error> {
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
