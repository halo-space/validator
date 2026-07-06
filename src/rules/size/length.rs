use crate::{Field, Rule};

#[derive(Debug)]
pub struct Length;

impl Rule for Length {
    fn check(&self, field: &Field<'_>) -> bool {
        if field.args().get("exact").is_some() {
            return super::satisfies(field, "exact", super::Relation::Eq);
        }

        if field.args().get("min").is_some()
            && !super::satisfies(field, "min", super::Relation::Gte)
        {
            return false;
        }

        if field.args().get("max").is_some()
            && !super::satisfies(field, "max", super::Relation::Lte)
        {
            return false;
        }

        true
    }
}
