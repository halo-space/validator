use validator::{Error, Validate, Validator};

struct Manual;

impl Validate for Manual {
    fn validate(&self, _validator: &Validator) -> Result<(), Error> {
        Ok(())
    }
}

fn main() {
    Validator::new().partial(&Manual, ["field"]).unwrap();
}
