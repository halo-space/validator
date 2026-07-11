use std::borrow::Cow;
use std::collections::HashMap;

use serde::Serialize;
use serde_json::json;
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

    #[validate(unique, dive(required))]
    tags: Vec<String>,

    #[validate(unique, dive(keys(max = 10), values(required)))]
    labels: HashMap<String, String>,

    #[validate(unique = "email")]
    members: Vec<Member>,

    #[validate(required, email)]
    backup_email: Email,
}

#[derive(Debug)]
struct Member {
    email: String,
}

#[derive(Debug, Validate)]
struct RuleShowcase {
    #[validate(ascii, containsany(value = "-_"), noneof("root", "admin"))]
    username: String,

    #[validate(oneof(1, 2, 3))]
    priority: u8,

    #[validate(noneof(-1, 0))]
    score: i32,

    #[validate(cidr)]
    network: String,

    #[validate(fqdn)]
    host: String,

    #[validate(hostname_rfc1123)]
    rfc_host: String,

    #[validate(port)]
    port: String,

    #[validate(uuid4)]
    request_id: String,

    #[validate(ulid)]
    public_id: String,

    #[validate(json)]
    metadata: String,

    #[validate(datetime)]
    created_at: String,
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
    fn check(&self, field: &Field<'_>) -> Result<bool, Error> {
        Ok(field
            .value()
            .string()
            .map(|value| {
                value
                    .chars()
                    .all(|ch| ch.is_ascii_lowercase() || ch.is_ascii_digit() || ch == '-')
            })
            .unwrap_or(false))
    }
}

#[derive(Debug, Serialize)]
struct SchemaUser {
    #[serde(rename = "user_name")]
    name: String,
    email: String,
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
        members: vec![
            Member {
                email: "first@example.com".to_owned(),
            },
            Member {
                email: "second@example.com".to_owned(),
            },
        ],
        backup_email: Email("backup@example.com".to_owned()),
    };

    validator.validate(&user)?;
    validator.partial(&user, ["name", "profile.display_name"])?;
    validator.except(&user, ["email"])?;
    validator.filter(&user, |namespace| {
        matches!(namespace.as_str(), "profile" | "profile.display_name")
    })?;
    validator.value(&"hello-rust", "required,slug")?;

    let showcase = RuleShowcase {
        username: "alice-dev".to_owned(),
        priority: 2,
        score: 10,
        network: "192.168.0.0/24".to_owned(),
        host: "api.example.com".to_owned(),
        rfc_host: "1.foo.com".to_owned(),
        port: "443".to_owned(),
        request_id: "550e8400-e29b-41d4-a716-446655440000".to_owned(),
        public_id: "01BX5ZZKBKACTAV9WEVGEMMVRZ".to_owned(),
        metadata: r#"{"ok":true}"#.to_owned(),
        created_at: "2026-07-08T12:30:00+08:00".to_owned(),
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

    let schema = Schema::from_yaml(
        r#"
fields:
  user_name:
    type: string
    rules:
      - required
      - length:
          min: 3
  email:
    type: string
    rules:
      - email
"#,
    )?;
    let schema_user = SchemaUser {
        name: "alice".to_owned(),
        email: "alice@example.com".to_owned(),
    };
    Validator::with_schema(schema.clone()).validate_serde(&schema_user)?;
    Validator::with_schema(schema).validate_map(&json!({
        "user_name": "alice",
        "email": "alice@example.com"
    }))?;

    let invalid = User {
        name: "al".to_owned(),
        email: "not-email".to_owned(),
        age: 42,
        profile: Profile {
            display_name: "Alice".to_owned(),
        },
        tags: vec!["rust".to_owned()],
        labels: HashMap::from([("source".to_owned(), "example".to_owned())]),
        members: vec![Member {
            email: "first@example.com".to_owned(),
        }],
        backup_email: Email("backup@example.com".to_owned()),
    };
    let error = validator.validate(&invalid).unwrap_err();
    let messages = validator::i18n::zh_cn().render(error.fields().unwrap());
    assert_eq!(messages[0].text, "name长度必须在3到20之间");

    println!("v1 example passed");
    Ok(())
}
