use validator::prelude::*;

#[derive(Debug)]
struct User {
    email: String,
}

#[derive(Debug, Validate)]
struct Request {
    #[validate(unique = ["email", "email"])]
    users: Vec<User>,
}

fn main() {}
