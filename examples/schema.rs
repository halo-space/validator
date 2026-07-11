use serde::Serialize;
use serde_json::json;
use validator::prelude::*;

#[derive(Debug, Serialize)]
struct User {
    #[serde(rename = "user_name")]
    name: String,
    email: String,
}

fn main() -> Result<(), Error> {
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

    validator.validate_serde(&User {
        name: "alice".to_owned(),
        email: "alice@example.com".to_owned(),
    })?;
    validator.validate_map(&json!({
        "user_name": "alice",
        "email": "alice@example.com"
    }))?;

    Ok(())
}
