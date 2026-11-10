use halo::validator::prelude::*;

struct Slug;

impl Rule for Slug {
    async fn check(&self, field: &Field<'_>) -> Result<bool, Error> {
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

#[tokio::main]
async fn main() -> Result<(), Error> {
    let validator = Validator::new()
        .rule("slug", Slug)?
        .alias("identifier", "required,slug")?;

    validator
        .validate(&Post {
            slug: "hello-rust".to_owned(),
        })
        .await?;
    validator.value(&"hello-rust", "identifier").await?;

    Ok(())
}
