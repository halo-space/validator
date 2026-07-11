use std::borrow::Cow;

use validator::prelude::*;

#[derive(Debug)]
struct Email(String);

impl Value for Email {
    fn kind(&self) -> Kind {
        Kind::String
    }

    fn declared_kind() -> Option<Kind> {
        Some(Kind::String)
    }

    fn required(&self) -> bool {
        !self.0.is_empty()
    }

    fn string(&self) -> Option<Cow<'_, str>> {
        Some(Cow::Borrowed(&self.0))
    }

    fn len(&self) -> Option<usize> {
        Some(self.0.chars().count())
    }
}

#[derive(Debug, Validate)]
struct Contact {
    #[validate(required, email)]
    email: Email,
}

fn main() -> Result<(), Error> {
    Validator::new().validate(&Contact {
        email: Email("alice@example.com".to_owned()),
    })
}
