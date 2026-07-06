use crate::{Field, Rule};

#[derive(Debug)]
pub struct Max;

impl Rule for Max {
    fn check(&self, field: &Field<'_>) -> bool {
        super::satisfies(field, "max", super::Relation::Lte)
    }
}
