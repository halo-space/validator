use crate::{Field, Rule};

#[derive(Debug)]
pub struct Ne;

impl Rule for Ne {
    fn check(&self, field: &Field<'_>) -> Result<bool, crate::Error> {
        super::not_equals(field)
    }
}
