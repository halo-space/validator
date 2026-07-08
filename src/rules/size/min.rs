use crate::{Field, Rule};

#[derive(Debug)]
pub struct Min;

impl Rule for Min {
    fn check(&self, field: &Field<'_>) -> Result<bool, crate::Error> {
        super::satisfies(field, "min", super::Relation::Gte)
    }
}
