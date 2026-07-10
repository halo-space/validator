use crate::{Field, Rule, Signature};

#[derive(Debug)]
pub struct Min;

impl Rule for Min {
    fn signature(&self) -> Signature {
        Signature::text("min")
    }

    fn check(&self, field: &Field<'_>) -> Result<bool, crate::Error> {
        super::satisfies(field, "min", super::Relation::Gte)
    }
}
