use validator::prelude::*;

#[derive(Debug)]
struct Profile {
    email: String,
}

#[derive(Debug, Validate)]
struct Request {
    profile: Profile,

    #[validate(eq_field = "profile.missing")]
    email: String,
}

fn main() {}
