use validator::prelude::*;

#[derive(Debug, Validate)]
struct Request {
    #[validate(eq_field = "profile.email")]
    email: String,
}

fn main() {}
