use crate::{Error, Validator};

pub trait Validate {
    fn validate(&self, validator: &Validator) -> Result<(), Error>;
}

#[doc(hidden)]
pub trait Selective {
    #[doc(hidden)]
    fn __validate_with_context(
        &self,
        validator: &Validator,
        context: &crate::__private::Context<'_>,
    ) -> Result<(), Error>;
}
