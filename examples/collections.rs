use std::collections::HashMap;

use validator::prelude::*;

#[derive(Debug, Validate)]
struct Team {
    #[validate(unique, dive(required))]
    tags: Vec<String>,

    #[validate(unique, dive(keys(max = 10), values(required)))]
    labels: HashMap<String, String>,

    #[validate(unique = "email")]
    members: Vec<Member>,
}

#[derive(Debug)]
struct Member {
    email: String,
}

fn main() -> Result<(), Error> {
    let team = Team {
        tags: vec!["rust".to_owned(), "validator".to_owned()],
        labels: HashMap::from([("source".to_owned(), "example".to_owned())]),
        members: vec![
            Member {
                email: "first@example.com".to_owned(),
            },
            Member {
                email: "second@example.com".to_owned(),
            },
        ],
    };

    Validator::new().validate(&team)
}
