use crate::{Field, Rule, Signature};

#[derive(Debug)]
pub struct Gte;

impl Rule for Gte {
    fn signature(&self) -> Signature {
        Signature::optional_text("value")
    }

    fn check(&self, field: &Field<'_>) -> Result<bool, crate::Error> {
        super::satisfies(field, "value", super::Relation::Gte)
    }
}
