use crate::{Field, Rule};

#[derive(Debug)]
pub struct Range;

impl Rule for Range {
    fn check(&self, field: &Field<'_>) -> bool {
        field.args().get("min").is_some()
            && field.args().get("max").is_some()
            && super::satisfies(field, "min", super::Relation::Gte)
            && super::satisfies(field, "max", super::Relation::Lte)
    }
}
