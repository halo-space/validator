use validator::prelude::*;

#[derive(Debug)]
struct Profile {
    email: String,
}

#[derive(Debug, Validate)]
struct EmptySegment {
    profile: Profile,

    #[validate(eq_field = "profile..email")]
    email: String,
}

#[derive(Debug, Validate)]
struct IndexedSegment {
    profile: Profile,

    #[validate(eq_field = "profile[0].email")]
    email: String,
}

fn main() {}
