use crate::{Field, Rule};

#[derive(Debug)]
pub struct Alpha;

impl Rule for Alpha {
    fn check(&self, field: &Field<'_>) -> bool {
        field.value().string().is_some_and(|value| {
            let value = value.as_ref();
            !value.is_empty() && value.chars().all(|ch| ch.is_ascii_alphabetic())
        })
    }
}
