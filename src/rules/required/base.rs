use crate::{Field, Rule};

#[derive(Debug)]
pub(crate) struct Required;

#[async_trait::async_trait(?Send)]
impl Rule for Required {
    fn validates_none(&self) -> bool {
        true
    }

    async fn check(&self, field: &Field<'_>) -> Result<bool, crate::Error> {
        Ok(field.value().required())
    }
}
