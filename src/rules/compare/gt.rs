use crate::{Field, Rule};

#[derive(Debug)]
pub struct Gt;

impl Rule for Gt {
    fn check(&self, field: &Field<'_>) -> Result<bool, crate::Error> {
        super::satisfies(field, "value", super::Relation::Gt)
    }
}
