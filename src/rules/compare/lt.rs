use crate::{Field, Rule};

#[derive(Debug)]
pub struct Lt;

impl Rule for Lt {
    fn check(&self, field: &Field<'_>) -> Result<bool, crate::Error> {
        super::satisfies(field, "value", super::Relation::Lt)
    }
}
