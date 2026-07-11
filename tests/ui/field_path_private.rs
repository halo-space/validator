use validator::prelude::*;

mod model {
    #[derive(Debug)]
    pub struct Profile {
        email: String,
    }

    impl Profile {
        pub fn new(email: impl Into<String>) -> Self {
            Self {
                email: email.into(),
            }
        }
    }
}

#[derive(Debug, Validate)]
struct Request {
    profile: model::Profile,

    #[validate(eq_field = "profile.email")]
    email: String,
}

fn main() {
    let _ = Request {
        profile: model::Profile::new("user@example.com"),
        email: "user@example.com".to_owned(),
    };
}
