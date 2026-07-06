use crate::{Field, Rule};

#[derive(Debug)]
pub struct Lt;

impl Rule for Lt {
    fn check(&self, field: &Field<'_>) -> bool {
        super::satisfies(field, "value", super::Relation::Lt)
    }
}
