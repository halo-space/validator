use crate::{Field, Rule, Signature};

#[derive(Debug)]
pub(crate) struct OneOfCi;

#[async_trait::async_trait(?Send)]
impl Rule for OneOfCi {
    fn signature(&self) -> Signature {
        Signature::list("values")
    }

    async fn check(&self, field: &Field<'_>) -> Result<bool, crate::Error> {
        Ok(super::contains_ignore_case(field).unwrap_or(false))
    }
}
