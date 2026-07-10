mod condition;
mod relation;

use crate::core::{Error, Registry};

pub(crate) fn load(registry: &mut Registry) -> Result<(), Error> {
    relation::load(registry)?;
    condition::load(registry)?;
    Ok(())
}
