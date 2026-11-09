use halo::validator::prelude::*;

#[derive(Debug)]
struct Profile {
    email: String,
}

#[derive(Debug)]
struct User {
    profile: Profile,
}

#[derive(Debug, Validate)]
struct Request {
    #[validate(unique = "profile.email")]
    users: Vec<User>,
}

fn main() {}
