use crate::{Field, Rule, Signature};

#[derive(Debug)]
pub struct Range;

impl Rule for Range {
    fn signature(&self) -> Signature {
        Signature::named(&["min", "max"], &["min", "max"])
    }

    fn validate_params(&self, field: &Field<'_>) -> Result<(), crate::Error> {
        super::validate(field, "min")?;
        super::validate(field, "max")?;
        super::validate_bounds(field, "range")
    }

    fn check(&self, field: &Field<'_>) -> Result<bool, crate::Error> {
        Ok(field.params().text("min").is_some()
            && field.params().text("max").is_some()
            && super::satisfies(field, "min", super::Relation::Gte)?
            && super::satisfies(field, "max", super::Relation::Lte)?)
    }
}
