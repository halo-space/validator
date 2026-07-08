use crate::{Field, Rule};

#[derive(Debug)]
pub struct Gte;

impl Rule for Gte {
    fn check(&self, field: &Field<'_>) -> Result<bool, crate::Error> {
        super::satisfies(field, "value", super::Relation::Gte)
    }
}
