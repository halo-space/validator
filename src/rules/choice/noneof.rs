use crate::{Field, Rule};

#[derive(Debug)]
pub struct NoneOf;

impl Rule for NoneOf {
    fn check(&self, field: &Field<'_>) -> bool {
        super::contains(field).is_some_and(|contains| !contains)
    }
}
