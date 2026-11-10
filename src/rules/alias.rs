use crate::core::{Error, Registry};

pub(crate) fn load(_registry: &mut Registry) -> Result<(), Error> {
    #[cfg(feature = "format")]
    _registry.alias("iscolor", "hexcolor|rgb|rgba|hsl|hsla|cmyk")?;
    Ok(())
}
