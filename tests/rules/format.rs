
#[derive(Debug, Validate)]
struct Formats {
    #[validate(email)]
    email: String,

    #[validate(url)]
    source_url: String,

    #[validate(uri)]
    urn: String,

    #[validate(regex(pattern = "^[a-z0-9-]+$"))]
    slug: String,
}

#[test]
fn email_url_regex_pass() {
    let value = Formats {
        email: "team@example.com".to_owned(),
        source_url: "https://example.com/posts/1".to_owned(),
        urn: "urn:isbn:0451450523".to_owned(),
        slug: "hello-rust-2024".to_owned(),
    };

    Validator::new().validate(&value).unwrap();
}

#[test]
fn email_url_regex_fail() {
    let value = Formats {
        email: "not-email".to_owned(),
        source_url: "example.com/posts/1".to_owned(),
        urn: "not uri".to_owned(),
        slug: "Hello Rust".to_owned(),
    };

    let fields = Validator::new()
        .validate(&value)
        .unwrap_err()
        .into_fields()
        .unwrap();

    assert_eq!(fields.len(), 4);
    assert_eq!(fields[0].rule(), "email");
    assert_eq!(fields[1].rule(), "url");
    assert_eq!(fields[2].rule(), "uri");
    assert_eq!(fields[3].rule(), "regex");
    assert_eq!(fields[3].params().text("pattern"), Some("^[a-z0-9-]+$"));
}

#[test]
fn email_rejects_local_dot_and_domain_label_boundaries() {
    for email in [
        ".user@example.com",
        "user..name@example.com",
        "user@-example.com",
    ] {
        let fields = Validator::new()
            .value(&email, "email")
            .unwrap_err()
            .into_fields()
            .unwrap();

        assert_eq!(fields[0].rule(), "email", "unexpectedly accepted {email}");
    }
}

#[test]
fn repeated_dynamic_regex_validation_preserves_errors() {
    let validator = Validator::new();
    let expression = r#"regex(pattern="^[a-z0-9-]+$")"#;

    validator.value(&"hello-rust", expression).unwrap();
    let first = validator
        .value(&"Hello Rust", expression)
        .unwrap_err()
        .into_fields()
        .unwrap();
    let second = validator
        .value(&"Hello Rust", expression)
        .unwrap_err()
        .into_fields()
        .unwrap();

    assert_eq!(first, second);
}
