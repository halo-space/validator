use crate::{Field, Rule, Signature};

#[derive(Debug)]
pub struct Length;

impl Rule for Length {
    fn signature(&self) -> Signature {
        Signature::named(&["min", "max", "exact"], &[])
    }

    fn validate_params(&self, field: &Field<'_>) -> Result<(), crate::Error> {
        let exact = field.params().text("exact").is_some();
        let min = field.params().text("min").is_some();
        let max = field.params().text("max").is_some();

        if !exact && !min && !max {
            return Err(crate::Error::InvalidRuleExpression {
                expression: "length".to_owned(),
                reason: "length requires 'exact', 'min', or 'max'".to_owned(),
            });
        }
        if exact && (min || max) {
            return Err(crate::Error::InvalidRuleExpression {
                expression: "length".to_owned(),
                reason: "length 'exact' cannot be combined with 'min' or 'max'".to_owned(),
            });
        }

        for name in ["min", "max", "exact"] {
            if field.params().text(name).is_some() {
                super::validate(field, name)?;
            }
        }
        if min && max {
            super::validate_bounds(field, "length")?;
        }
        Ok(())
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
