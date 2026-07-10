use std::collections::HashMap;

use validator::prelude::*;

#[derive(Debug)]
struct User {
    email: String,
}

#[derive(Debug, Validate)]
struct Request {
    #[validate(unique = "email")]
    users: HashMap<String, User>,
}

fn main() {}
