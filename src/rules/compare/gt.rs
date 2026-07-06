use crate::{Field, Rule};

#[derive(Debug)]
pub struct Gt;

impl Rule for Gt {
    fn check(&self, field: &Field<'_>) -> bool {
        super::satisfies(field, "value", super::Relation::Gt)
    }
}
