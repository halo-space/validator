use crate::{Field, Rule};

#[derive(Debug)]
pub struct Uppercase;

impl Rule for Uppercase {
    fn check(&self, field: &Field<'_>) -> bool {
        field.value().string().is_some_and(|value| {
            let value = value.as_ref();
            !value.is_empty() && value == value.to_uppercase()
        })
    }
}
