use std::borrow::Cow;
use std::collections::HashMap;

use validator::prelude::*;

#[derive(Debug, Validate)]
struct User {
    #[validate(alias = "username")]
    name: String,

    #[validate(omitempty, email)]
    email: String,

    #[validate(gte = 0, lte = 130)]
    age: u8,

    #[validate(nested)]
    profile: Profile,

    #[validate(dive(required))]
    tags: Vec<String>,

    #[validate(dive(keys(max = 10), values(required)))]
    labels: HashMap<String, String>,

    #[validate(required, email)]
    backup_email: Email,
}

#[derive(Debug, Validate)]
struct RuleShowcase {
    #[validate(ascii, containsany(value = "-_"), noneof("root", "admin"))]
    username: String,

    #[validate(oneof(1, 2, 3))]
    priority: u8,

    #[validate(noneof(-1, 0))]
    score: i32,
}

#[derive(Debug, Validate)]
struct Profile {
    #[validate(required)]
    display_name: String,
}

#[derive(Debug)]
struct Email(String);

impl Value for Email {
    fn kind(&self) -> Kind {
        Kind::String
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
struct Event {
    start_at: i64,

    #[validate(gt_field = "start_at")]
    end_at: i64,
}

#[derive(Debug, Validate)]
#[validate(check = "validate_draft")]
struct Draft {
    name: String,
    title: String,
}

fn validate_draft(draft: &Draft, valid: &mut validator::valid::Valid<'_>) {
    if draft.name.is_empty() && draft.title.is_empty() {
        valid
            .field("name")
            .rule("required_without")
            .param("field", "title")
            .push();
        valid
            .field("title")
            .rule("required_without")
            .param("field", "name")
            .push();
    }
}

struct Slug;

impl Rule for Slug {
    fn check(&self, field: &Field<'_>) -> bool {
        field
            .value()
            .string()
            .map(|value| {
                value
                    .chars()
                    .all(|ch| ch.is_ascii_lowercase() || ch.is_ascii_digit() || ch == '-')
            })
            .unwrap_or(false)
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let validator = Validator::new()
        .alias("username", "required,length(min=3,max=20)")?
        .rule("slug", Slug)?;

    let user = User {
        name: "alice".to_owned(),
        email: "alice@example.com".to_owned(),
        age: 42,
        profile: Profile {
            display_name: "Alice".to_owned(),
        },
        tags: vec!["rust".to_owned(), "validator".to_owned()],
        labels: HashMap::from([("source".to_owned(), "example".to_owned())]),
        backup_email: Email("backup@example.com".to_owned()),
    };

    validator.validate(&user)?;
    validator.value(&"hello-rust", "required,slug")?;

    let showcase = RuleShowcase {
        username: "alice-dev".to_owned(),
        priority: 2,
        score: 10,
    };
    validator.validate(&showcase)?;
    validator.value(&4_u8, "noneof(1,2,3)")?;

    let event = Event {
        start_at: 10,
        end_at: 20,
    };
    validator.validate(&event)?;

    let draft = Draft {
        name: "Rust".to_owned(),
        title: String::new(),
    };
    validator.validate(&draft)?;

    let invalid = User {
        name: "al".to_owned(),
        email: "not-email".to_owned(),
        age: 42,
        profile: Profile {
            display_name: "Alice".to_owned(),
        },
        tags: vec!["rust".to_owned()],
        labels: HashMap::from([("source".to_owned(), "example".to_owned())]),
        backup_email: Email("backup@example.com".to_owned()),
    };
    let error = validator.validate(&invalid).unwrap_err();
    let messages = validator::i18n::zh_cn().render(error.fields().unwrap());
    assert_eq!(messages[0].text, "name长度必须在3到20之间");

    println!("v1 example passed");
    Ok(())
}
