use crate::{Field, Rule, Signature};

#[derive(Debug)]
pub(crate) struct Excludes;

impl Rule for Excludes {
    fn signature(&self) -> Signature {
        Signature::text("value")
    }

    fn check(&self, field: &Field<'_>) -> Result<bool, crate::Error> {
        Ok(super::value_and_expected(field, "value")
            .is_some_and(|(value, expected)| !value.contains(expected)))
    }
}
