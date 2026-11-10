use crate::{Field, Rule, Signature};

#[derive(Debug)]
pub(crate) struct NeIgnoreCase;

#[async_trait::async_trait(?Send)]
impl Rule for NeIgnoreCase {
    fn signature(&self) -> Signature {
        Signature::text("value")
    }

    async fn check(&self, field: &Field<'_>) -> Result<bool, crate::Error> {
        Ok(super::eq_ignore_case::compare(field).is_some_and(|equal| !equal))
    }
}
