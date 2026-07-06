use crate::{Field, Rule};

#[derive(Debug)]
pub struct Gte;

impl Rule for Gte {
    fn check(&self, field: &Field<'_>) -> bool {
        super::satisfies(field, "value", super::Relation::Gte)
    }
}
