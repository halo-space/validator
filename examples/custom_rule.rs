use validator::prelude::*;

struct Slug;

impl Rule for Slug {
    fn check(&self, field: &Field<'_>) -> Result<bool, Error> {
        Ok(field.value().string().is_some_and(|value| {
            value
                .chars()
                .all(|character| character.is_ascii_lowercase() || character == '-')
        }))
    }
}

#[derive(Debug, Validate)]
struct Post {
    #[validate(slug)]
    slug: String,
}

fn main() -> Result<(), Error> {
    let validator = Validator::new()
        .rule("slug", Slug)?
        .alias("identifier", "required,slug")?;

    validator.validate(&Post {
        slug: "hello-rust".to_owned(),
    })?;
    validator.value(&"hello-rust", "identifier")?;

    Ok(())
}
