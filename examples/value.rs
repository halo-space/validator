use validator::prelude::*;

fn main() -> Result<(), Error> {
    let validator = Validator::new();

    validator.value(&"alice@example.com", "required,email")?;
    validator.value(&"192.168.0.0/24", "cidr")?;
    validator.value(&"550e8400-e29b-41d4-a716-446655440000", "uuid4")?;
    validator.value(&2_u8, "oneof(1,2,3)")?;

    let error = validator.value(&"not-email", "email").unwrap_err();
    let fields = error
        .fields()
        .expect("validation errors must contain fields");
    assert_eq!(fields[0].namespace().as_str(), "$value");
    assert_eq!(fields[0].rule(), "email");

    Ok(())
}
