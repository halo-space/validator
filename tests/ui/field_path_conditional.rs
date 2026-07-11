use validator::prelude::*;

#[derive(Debug)]
struct Profile {
    email: String,
}

#[derive(Debug, Validate)]
struct Request {
    profile: Profile,

    #[validate(required_with("profile.email"))]
    title: String,
}

fn main() {}
