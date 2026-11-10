use halo::validator::{Error, Validate, Validator};

struct Manual;

impl Validate for Manual {
    async fn validate(&self, _validator: &Validator) -> Result<(), Error> {
        Ok(())
    }
}

#[tokio::main]
async fn main() {
    Validator::new().partial(&Manual, ["field"]).await.unwrap();
}
