use crate::{Field, Rule};

#[derive(Debug)]
pub struct Required;

impl Rule for Required {
    fn check(&self, field: &Field<'_>) -> bool {
        field.value().required()
    }
}
