use crate::{Field, Rule};

#[derive(Debug)]
pub(crate) struct Required;

impl Rule for Required {
    fn validates_none(&self) -> bool {
        true
    }

    fn check(&self, field: &Field<'_>) -> Result<bool, crate::Error> {
        Ok(field.value().required())
    }
}
