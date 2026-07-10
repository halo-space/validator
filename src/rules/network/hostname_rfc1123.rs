use crate::{Field, Rule};

#[derive(Debug)]
pub struct HostnameRfc1123;

impl Rule for HostnameRfc1123 {
    fn check(&self, field: &Field<'_>) -> Result<bool, crate::Error> {
        Ok(field
            .value()
            .string()
            .is_some_and(|value| super::host::valid_rfc1123(value.as_ref())))
    }
}
