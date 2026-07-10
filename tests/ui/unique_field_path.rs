use validator::prelude::*;

#[derive(Debug)]
struct User {
    email: String,
}

#[derive(Debug, Validate)]
struct Request {
    #[validate(unique = "profile.email")]
    users: Vec<User>,
}

fn main() {}
