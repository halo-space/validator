use crate::{Field, Rule};

#[derive(Debug)]
pub(crate) struct IsDefault;

impl Rule for IsDefault {
    fn check(&self, field: &Field<'_>) -> Result<bool, crate::Error> {
        Ok(!field.value().required())
    }
}
