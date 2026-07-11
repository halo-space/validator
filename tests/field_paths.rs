use serde_json::json;
use validator::prelude::*;

fn fields(error: Error) -> Vec<FieldError> {
    error
        .into_fields()
        .unwrap_or_else(|| panic!("expected validation errors"))
}

#[derive(Debug)]
struct Contact {
    email: String,
}

#[derive(Debug)]
struct Profile {
    contact: Contact,
}

#[derive(Debug, Validate)]
struct NestedTarget {
    profile: Profile,

    #[validate(eq_field = "profile.contact.email")]
    email: String,
}

#[test]
fn derive_resolves_arbitrarily_deep_borrowed_targets() {
    let user = NestedTarget {
        profile: Profile {
            contact: Contact {
                email: "user@example.com".to_owned(),
            },
        },
        email: "user@example.com".to_owned(),
    };

    Validator::new().validate(&user).unwrap();

    let target = validator::__private::Access::field(&user, "profile.contact.email")
        .expect("referenced path must be available");
    assert_eq!(target.name(), "profile.contact.email");
    assert_eq!(target.value().string().as_deref(), Some("user@example.com"));
    assert!(validator::__private::Access::field(&user, "profile").is_none());
}

#[test]
fn derive_reports_the_full_target_path() {
    let error = Validator::new()
        .validate(&NestedTarget {
            profile: Profile {
                contact: Contact {
                    email: "expected@example.com".to_owned(),
                },
            },
            email: "actual@example.com".to_owned(),
        })
        .unwrap_err();
    let fields = fields(error);

    assert_eq!(fields.len(), 1);
    assert_eq!(fields[0].field(), "email");
    assert_eq!(fields[0].rule(), "eq_field");
    assert_eq!(
        fields[0].params().text("compare"),
        Some("profile.contact.email")
    );
}

#[derive(Debug)]
struct OptionalContact {
    email: Option<String>,
}

#[derive(Debug)]
struct OptionalProfile {
    contact: Option<OptionalContact>,
}

#[derive(Debug, Validate)]
struct OptionalTarget {
    profile: Option<OptionalProfile>,

    #[validate(ne_field = "profile.contact.email")]
    email: String,
}

fn optional_target(profile: Option<OptionalProfile>) -> Result<(), Error> {
    Validator::new().validate(&OptionalTarget {
        profile,
        email: "current@example.com".to_owned(),
    })
}

#[test]
fn derive_borrows_option_segments_and_treats_none_as_missing() {
    optional_target(Some(OptionalProfile {
        contact: Some(OptionalContact {
            email: Some("other@example.com".to_owned()),
        }),
    }))
    .unwrap();

    for profile in [
        None,
        Some(OptionalProfile { contact: None }),
        Some(OptionalProfile {
            contact: Some(OptionalContact { email: None }),
        }),
    ] {
        let fields = fields(optional_target(profile).unwrap_err());
        assert_eq!(fields.len(), 1);
        assert_eq!(fields[0].rule(), "ne_field");
        assert_eq!(
            fields[0].params().text("compare"),
            Some("profile.contact.email")
        );
    }
}

#[derive(Debug)]
struct RawProfile {
    r#type: String,
}

#[derive(Debug, Validate)]
struct RawTarget {
    profile: RawProfile,

    #[validate(eq_field = "profile.type")]
    kind: String,
}

#[test]
fn derive_uses_canonical_raw_identifier_segments() {
    Validator::new()
        .validate(&RawTarget {
            profile: RawProfile {
                r#type: "admin".to_owned(),
            },
            kind: "admin".to_owned(),
        })
        .unwrap();
}

#[derive(Debug)]
struct BorrowedProfile<'a> {
    email: &'a str,
}

#[derive(Debug, Validate)]
struct BorrowedTarget<'a> {
    profile: &'a BorrowedProfile<'a>,

    #[validate(eq_field = "profile.email")]
    email: &'a str,
}

#[test]
fn derive_resolves_borrowed_intermediate_and_terminal_fields() {
    let profile = BorrowedProfile {
        email: "user@example.com",
    };

    Validator::new()
        .validate(&BorrowedTarget {
            profile: &profile,
            email: "user@example.com",
        })
        .unwrap();
}

#[derive(Debug, Validate)]
struct CheckedProfile {
    #[validate(required)]
    name: String,
    email: String,
}

#[derive(Debug, Validate)]
struct WithoutNested {
    profile: CheckedProfile,

    #[validate(eq_field = "profile.email")]
    email: String,
}

#[test]
fn path_access_does_not_trigger_nested_validation() {
    Validator::new()
        .validate(&WithoutNested {
            profile: CheckedProfile {
                name: String::new(),
                email: "user@example.com".to_owned(),
            },
            email: "user@example.com".to_owned(),
        })
        .unwrap();
}

#[derive(Debug, Validate)]
struct ScopedProfile {
    contact: Contact,

    #[validate(eq_field = "contact.email")]
    email: String,
}

#[derive(Debug, Validate)]
struct ScopedAccount {
    #[validate(nested)]
    profile: ScopedProfile,
}

#[test]
fn derive_path_is_relative_to_the_struct_that_declares_the_rule() {
    Validator::new()
        .validate(&ScopedAccount {
            profile: ScopedProfile {
                contact: Contact {
                    email: "user@example.com".to_owned(),
                },
                email: "user@example.com".to_owned(),
            },
        })
        .unwrap();
}

#[derive(Debug)]
struct NumericProfile {
    score: i64,
}

#[derive(Debug, Validate)]
struct StrictKindTarget {
    profile: NumericProfile,

    #[validate(eq_field = "profile.score")]
    score: i32,
}

#[test]
fn nested_target_keeps_strict_concrete_kind_matching() {
    let fields = fields(
        Validator::new()
            .validate(&StrictKindTarget {
                profile: NumericProfile { score: 7 },
                score: 7,
            })
            .unwrap_err(),
    );

    assert_eq!(fields.len(), 1);
    assert_eq!(fields[0].rule(), "eq_field");
}

#[derive(Debug)]
struct SamePath;

impl Rule for SamePath {
    fn signature(&self) -> Signature {
        Signature::field("compare")
    }

    fn check(&self, field: &Field<'_>) -> Result<bool, Error> {
        let Some(target) = field.params().text("compare") else {
            return Ok(false);
        };

        Ok(field.sibling(target).and_then(Value::string) == field.value().string())
    }
}

#[derive(Debug, Validate)]
struct CustomTarget {
    profile: Profile,

    #[validate(same_field = "profile.contact.email")]
    email: String,
}

#[test]
fn custom_field_rule_can_use_a_nested_target() {
    Validator::new()
        .rule("same_field", SamePath)
        .unwrap()
        .validate(&CustomTarget {
            profile: Profile {
                contact: Contact {
                    email: "user@example.com".to_owned(),
                },
            },
            email: "user@example.com".to_owned(),
        })
        .unwrap();
}

fn nested_schema(rule: &str) -> Schema {
    Schema::from_yaml(format!(
        r#"
fields:
  profile:
    type: object
    fields:
      email:
        type: string
  email:
    type: string
    rules:
      - {rule}
"#
    ))
    .unwrap()
}

#[test]
fn schema_resolves_nested_targets_from_rules_and_aliases() {
    let schema = nested_schema("same_profile_email");
    let validator = Validator::with_schema(schema)
        .alias("same_profile_email", "eq_field=profile.email")
        .unwrap();

    validator
        .validate_map(&json!({
            "profile": { "email": "user@example.com" },
            "email": "user@example.com"
        }))
        .unwrap();

    let fields = fields(
        validator
            .validate_map(&json!({
                "profile": { "email": "expected@example.com" },
                "email": "actual@example.com"
            }))
            .unwrap_err(),
    );
    assert_eq!(fields.len(), 1);
    assert_eq!(fields[0].rule(), "same_profile_email");
    assert_eq!(fields[0].reason(), "eq_field");
    assert_eq!(fields[0].params().text("compare"), Some("profile.email"));
}

#[test]
fn schema_custom_field_rule_can_use_a_nested_target() {
    Validator::with_schema(nested_schema("same_field: profile.email"))
        .rule("same_field", SamePath)
        .unwrap()
        .validate_map(&json!({
            "profile": { "email": "user@example.com" },
            "email": "user@example.com"
        }))
        .unwrap();
}

#[test]
fn schema_rejects_literal_field_that_conflicts_with_nested_target() {
    let schema = Schema::from_yaml(
        r#"
fields:
  profile:
    type: object
    fields:
      email:
        type: string
  "profile.email":
    type: string
  email:
    type: string
    rules:
      - eq_field: profile.email
"#,
    )
    .unwrap();

    let error = Validator::with_schema(schema)
        .validate_map(&json!({
            "profile": { "email": "nested@example.com" },
            "profile.email": "literal@example.com",
            "email": "nested@example.com"
        }))
        .unwrap_err();

    assert!(matches!(
        error,
        Error::InvalidSchema { reason }
            if reason.contains("conflicts with a literal field of the same name")
    ));
}

#[test]
fn schema_direct_target_accepts_serialized_field_name() {
    let schema = Schema::from_yaml(
        r#"
fields:
  source-url:
    type: string
  mirror:
    type: string
    rules:
      - eq_field: source-url
"#,
    )
    .unwrap();

    Validator::with_schema(schema)
        .validate_map(&json!({
            "source-url": "https://example.com",
            "mirror": "https://example.com"
        }))
        .unwrap();
}

#[test]
fn schema_uses_canonical_raw_identifier_segments() {
    let schema = Schema::from_yaml(
        r#"
fields:
  profile:
    type: object
    fields:
      type:
        type: string
  kind:
    type: string
    rules:
      - eq_field: profile.type
"#,
    )
    .unwrap();

    Validator::with_schema(schema)
        .validate_map(&json!({ "profile": { "type": "admin" }, "kind": "admin" }))
        .unwrap();
}

#[test]
fn schema_treats_missing_and_null_path_segments_as_missing_targets() {
    let validator = Validator::with_schema(nested_schema("ne_field: profile.email"));

    for data in [
        json!({ "email": "user@example.com" }),
        json!({ "profile": null, "email": "user@example.com" }),
        json!({ "profile": {}, "email": "user@example.com" }),
        json!({ "profile": { "email": null }, "email": "user@example.com" }),
    ] {
        let fields = fields(validator.validate_map(&data).unwrap_err());
        assert_eq!(fields.len(), 1);
        assert_eq!(fields[0].rule(), "ne_field");
    }
}

#[test]
fn schema_keeps_declared_terminal_kind_for_nested_targets() {
    let schema = Schema::from_yaml(
        r#"
fields:
  profile:
    type: object
    fields:
      score:
        type: uint
  score:
    type: integer
    rules:
      - eq_field: profile.score
"#,
    )
    .unwrap();
    let fields = fields(
        Validator::with_schema(schema)
            .validate_map(&json!({ "profile": { "score": 7 }, "score": 7 }))
            .unwrap_err(),
    );

    assert_eq!(fields.len(), 1);
    assert_eq!(fields[0].rule(), "eq_field");
}

#[test]
fn schema_invalid_intermediate_data_reports_type_and_missing_target() {
    let fields = fields(
        Validator::with_schema(nested_schema("eq_field: profile.email"))
            .validate_map(&json!({
                "profile": "not-an-object",
                "email": "user@example.com"
            }))
            .unwrap_err(),
    );
    let mut rules = fields.iter().map(FieldError::rule).collect::<Vec<_>>();
    rules.sort_unstable();

    assert_eq!(rules, vec!["eq_field", "type"]);
}

#[test]
fn schema_rejects_invalid_or_undeclared_paths() {
    for target in ["profile.missing", "profile..email", "profile[0].email"] {
        let schema = nested_schema(&format!("eq_field: {target}"));
        let error = Validator::with_schema(schema)
            .validate_map(&json!({
                "profile": { "email": "user@example.com" },
                "email": "user@example.com"
            }))
            .unwrap_err();

        assert!(matches!(error, Error::InvalidSchema { .. }));
    }
}

#[test]
fn schema_rejects_scalar_and_array_intermediate_segments() {
    for ty in ["string", "array"] {
        let schema = Schema::from_yaml(format!(
            r#"
fields:
  profile:
    type: {ty}
  email:
    type: string
    rules:
      - eq_field: profile.email
"#
        ))
        .unwrap();
        let error = Validator::with_schema(schema)
            .validate_map(&json!({ "email": "user@example.com" }))
            .unwrap_err();

        assert!(matches!(error, Error::InvalidSchema { .. }));
    }
}

#[test]
fn schema_conditional_rules_keep_direct_field_targets() {
    let schema = Schema::from_yaml(
        r#"
fields:
  profile:
    type: object
    fields:
      email:
        type: string
  title:
    type: string
    rules:
      - required_with: [profile.email]
"#,
    )
    .unwrap();
    let error = Validator::with_schema(schema)
        .validate_map(&json!({ "profile": { "email": "user@example.com" } }))
        .unwrap_err();

    assert!(matches!(
        error,
        Error::InvalidSchema { reason }
            if reason.contains("references undeclared field 'profile.email'")
    ));
}

#[test]
fn schema_paths_are_relative_to_each_object_scope() {
    let schema = Schema::from_yaml(
        r#"
fields:
  account:
    type: object
    fields:
      profile:
        type: object
        fields:
          email:
            type: string
      email:
        type: string
        rules:
          - eq_field: profile.email
  users:
    type: array
    fields:
      profile:
        type: object
        fields:
          email:
            type: string
      email:
        type: string
        rules:
          - eq_field: profile.email
"#,
    )
    .unwrap();

    Validator::with_schema(schema)
        .validate_map(&json!({
            "account": {
                "profile": { "email": "account@example.com" },
                "email": "account@example.com"
            },
            "users": [{
                "profile": { "email": "user@example.com" },
                "email": "user@example.com"
            }]
        }))
        .unwrap();
}
