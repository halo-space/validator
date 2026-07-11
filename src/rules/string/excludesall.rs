use crate::{Field, Rule, Signature};

#[derive(Debug)]
pub(crate) struct ExcludesAll;

impl Rule for ExcludesAll {
    fn signature(&self) -> Signature {
        Signature::text("value")
    }

    fn check(&self, field: &Field<'_>) -> Result<bool, crate::Error> {
        Ok(super::value_and_expected(field, "value")
            .is_some_and(|(value, expected)| !value.chars().any(|ch| expected.contains(ch))))
    }
}
