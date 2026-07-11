use validator::prelude::*;

mod model {
    #[derive(Debug)]
    pub struct Profile {
        email: String,
    }

    #[derive(Debug)]
    pub struct User {
        pub tenant_id: u64,
        pub profile: Profile,
    }
}

#[derive(Debug, Validate)]
struct Request {
    #[validate(unique = ["tenant_id", "profile.email"])]
    users: Vec<model::User>,
}

fn main() {}
