use crate::core::{Aliases, Error};

pub(crate) fn load(aliases: &mut Aliases) -> Result<(), Error> {
    aliases.insert("iscolor", "hexcolor|rgb|rgba|hsl|hsla|cmyk")?;
    Ok(())
}
