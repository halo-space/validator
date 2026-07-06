use validator::prelude::*;

#[derive(Debug, Validate)]
struct User {
    #[validate(required, length(min = 3, max = 20))]
    name: String,
}

#[test]
fn valid_struct_passes() {
    let user = User {
        name: "alice".to_owned(),
    };

    Validator::new().validate(&user).unwrap();
}

#[test]
fn required_reports_field_error() {
    let user = User {
        name: String::new(),
    };

    let errors = Validator::new().validate(&user).unwrap_err();
    let fields = errors.into_vec();

    assert_eq!(fields.len(), 2);
    assert_eq!(fields[0].namespace().as_str(), "User.name");
    assert_eq!(fields[0].field(), "name");
    assert_eq!(fields[0].rule(), "required");
    assert_eq!(fields[0].actual_rule(), "required");
}

#[test]
fn length_reports_args() {
    let user = User {
        name: "al".to_owned(),
    };

    let errors = Validator::new().validate(&user).unwrap_err();
    let fields = errors.into_vec();

    assert_eq!(fields.len(), 1);
    assert_eq!(fields[0].rule(), "length");
    assert_eq!(fields[0].args().get("min"), Some("3"));
    assert_eq!(fields[0].args().get("max"), Some("20"));
}

#[derive(Debug, Validate)]
struct OptionalUser {
    #[validate(length(min = 3, max = 20))]
    nickname: Option<String>,

    #[validate(required)]
    email: Option<String>,
}

#[test]
fn option_none_skips_non_required_rules() {
    let user = OptionalUser {
        nickname: None,
        email: Some("x@example.com".to_owned()),
    };

    Validator::new().validate(&user).unwrap();
}

#[test]
fn option_none_fails_required() {
    let user = OptionalUser {
        nickname: None,
        email: None,
    };

    let errors = Validator::new().validate(&user).unwrap_err();
    let fields = errors.into_vec();

    assert_eq!(fields.len(), 1);
    assert_eq!(fields[0].namespace().as_str(), "OptionalUser.email");
    assert_eq!(fields[0].rule(), "required");
}

#[derive(Debug, Validate)]
struct AliasUser {
    #[validate(alias = "username")]
    name: String,
}

#[test]
fn alias_expands_to_rules() -> Result<(), Box<dyn std::error::Error>> {
    let user = AliasUser {
        name: "al".to_owned(),
    };

    let errors = Validator::new()
        .alias("username", "required,length(min=3,max=20)")?
        .validate(&user)
        .unwrap_err();
    let fields = errors.into_vec();

    assert_eq!(fields.len(), 1);
    assert_eq!(fields[0].rule(), "username");
    assert_eq!(fields[0].actual_rule(), "length");
    assert_eq!(fields[0].args().get("min"), Some("3"));

    Ok(())
}

#[derive(Debug, Validate)]
struct SlugPost {
    #[validate(alias = "slug_alias")]
    slug: String,
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

#[test]
fn custom_rule_chain_works() -> Result<(), Box<dyn std::error::Error>> {
    let post = SlugPost {
        slug: "Hello World".to_owned(),
    };

    let errors = Validator::new()
        .alias("slug_alias", "slug")?
        .rule("slug", Slug)?
        .validate(&post)
        .unwrap_err();
    let fields = errors.into_vec();

    assert_eq!(fields.len(), 1);
    assert_eq!(fields[0].rule(), "slug_alias");
    assert_eq!(fields[0].actual_rule(), "slug");

    Ok(())
}
