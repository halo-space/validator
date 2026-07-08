use crate::{Field, Rule};

#[derive(Debug)]
pub struct Range;

impl Rule for Range {
    fn check(&self, field: &Field<'_>) -> Result<bool, crate::Error> {
        Ok(field.params().get("min").is_some()
            && field.params().get("max").is_some()
            && super::satisfies(field, "min", super::Relation::Gte)?
            && super::satisfies(field, "max", super::Relation::Lte)?)
    }
}
