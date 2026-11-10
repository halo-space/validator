use crate::{Field, Rule};

#[derive(Debug)]
pub(crate) struct PrintAscii;

#[async_trait::async_trait(?Send)]
impl Rule for PrintAscii {
    async fn check(&self, field: &Field<'_>) -> Result<bool, crate::Error> {
        Ok(field
            .value()
            .string()
            .is_some_and(|value| value.bytes().all(|byte| (0x20..=0x7e).contains(&byte))))
    }
}
