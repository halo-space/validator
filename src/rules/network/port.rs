use crate::{Field, Rule};

#[derive(Debug)]
pub(crate) struct Port;

impl Rule for Port {
    fn check(&self, field: &Field<'_>) -> Result<bool, crate::Error> {
        Ok(field
            .value()
            .string()
            .and_then(|value| value.parse::<u16>().ok())
            .is_some_and(|port| port > 0))
    }
}
