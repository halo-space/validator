use crate::core::{Error, Registry};

pub(crate) fn load(registry: &mut Registry) -> Result<(), Error> {
    registry.alias("iscolor", "hexcolor|rgb|rgba|hsl|hsla|cmyk")?;
    Ok(())
}
