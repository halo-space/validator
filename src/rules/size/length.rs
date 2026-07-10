use crate::{Field, Rule, Signature};

#[derive(Debug)]
pub struct Length;

impl Rule for Length {
    fn signature(&self) -> Signature {
        Signature::named(&["min", "max", "exact"], &[])
    }

    fn check(&self, field: &Field<'_>) -> Result<bool, crate::Error> {
        if field.params().text("exact").is_some() {
            return super::satisfies(field, "exact", super::Relation::Eq);
        }

        if field.params().text("min").is_some()
            && !super::satisfies(field, "min", super::Relation::Gte)?
        {
            return Ok(false);
        }

        if field.params().text("max").is_some()
            && !super::satisfies(field, "max", super::Relation::Lte)?
        {
            return Ok(false);
        }

        Ok(true)
    }
}
