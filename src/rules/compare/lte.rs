use crate::{Field, Rule};

#[derive(Debug)]
pub struct Lte;

impl Rule for Lte {
    fn check(&self, field: &Field<'_>) -> Result<bool, crate::Error> {
        super::satisfies(field, "value", super::Relation::Lte)
    }
}
