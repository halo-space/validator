use crate::{Field, Rule};

#[derive(Debug)]
pub struct Ne;

impl Rule for Ne {
    fn check(&self, field: &Field<'_>) -> bool {
        super::not_equals(field)
    }
}
