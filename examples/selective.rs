use validator::prelude::*;

#[derive(Debug, Validate)]
struct User {
    #[validate(required)]
    name: String,

    #[validate(required, email)]
    email: String,

    #[validate(nested)]
    profile: Profile,
}

#[derive(Debug, Validate)]
struct Profile {
    #[validate(required)]
    display_name: String,
}

fn main() -> Result<(), Error> {
    let validator = Validator::new();
    let user = User {
        name: "alice".to_owned(),
        email: String::new(),
        profile: Profile {
            display_name: "Alice".to_owned(),
        },
    };

    let error = validator.partial(&user, ["email"]).unwrap_err();
    let fields = error
        .fields()
        .expect("validation errors must contain fields");
    assert_eq!(fields.len(), 2);
    assert_eq!(fields[0].namespace().as_str(), "User.email");
    assert_eq!(fields[0].rule(), "required");
    assert_eq!(fields[1].rule(), "email");

    validator.except(&user, ["email"])?;
    validator.filter(&user, |namespace| namespace.as_str().starts_with("profile"))?;

    Ok(())
}
