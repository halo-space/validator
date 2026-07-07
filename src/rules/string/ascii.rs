use crate::{Field, Rule};

#[derive(Debug)]
pub struct Ascii;

impl Rule for Ascii {
    fn check(&self, field: &Field<'_>) -> bool {
        field.value().string().is_some_and(|value| value.is_ascii())
    }
}
