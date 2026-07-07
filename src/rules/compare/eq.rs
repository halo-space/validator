use crate::{Field, Rule};

#[derive(Debug)]
pub struct Eq;

impl Rule for Eq {
    fn check(&self, field: &Field<'_>) -> bool {
        super::equals(field)
    }
}
