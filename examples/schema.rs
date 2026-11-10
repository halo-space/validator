use halo::validator::prelude::*;
use serde::Serialize;
use serde_json::json;

#[derive(Debug, Serialize)]
struct User {
    #[serde(rename = "user_name")]
    name: String,
    email: String,
}

#[tokio::main]
async fn main() -> Result<(), Error> {
    let schema = Schema::from_yaml(
        r#"
fields:
  user_name:
    type: string
    rules: required,length(min=3)
  email:
    type: string
    rules: required,email
"#,
    )?;
    let validator = Validator::with_schema(schema);

    validator
        .check_serde(&User {
            name: "alice".to_owned(),
            email: "alice@example.com".to_owned(),
        })
        .await?;
    validator
        .check_map(&json!({
            "user_name": "alice",
            "email": "alice@example.com"
        }))
        .await?;

    Ok(())
}
