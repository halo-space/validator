use crate::core::{Error, Registry};

pub(crate) fn load(aliases: &mut Registry) -> Result<(), Error> {
    aliases.alias("iscolor", "hexcolor|rgb|rgba|hsl|hsla|cmyk")?;
    Ok(())
}
