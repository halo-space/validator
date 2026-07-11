use validator::prelude::*;

#[derive(Debug, Validate)]
struct User {
    #[validate(required, length(min = 3, max = 20))]
    name: String,

    #[validate(omitempty, email)]
    email: String,

    #[validate(gte = 0, lte = 130)]
    age: u8,
}

fn main() -> Result<(), Error> {
    let validator = Validator::new();
    validator.validate(&User {
        name: "alice".to_owned(),
        email: "alice@example.com".to_owned(),
        age: 42,
    })?;

    let error = validator
        .validate(&User {
            name: "al".to_owned(),
            email: "not-email".to_owned(),
            age: 42,
        })
        .unwrap_err();
    let fields = error
        .fields()
        .expect("validation errors must contain fields");
    assert_eq!(fields.len(), 2);
    assert_eq!(fields[0].field(), "name");
    assert_eq!(fields[1].field(), "email");

    Ok(())
}
