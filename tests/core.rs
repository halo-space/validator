use std::collections::{BTreeMap, HashMap};
use std::time::{Duration, SystemTime};

use serde::Serialize;
use serde_json::json;
use validator::prelude::*;

fn fields(error: validator::Error) -> Vec<validator::FieldError> {
    error
        .into_fields()
        .unwrap_or_else(|| panic!("expected validation errors"))
}

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
fn derive_exposes_validated_field_access() {
    let user = User {
        name: "alice".to_owned(),
    };

    let field = validator::__private::Access::field(&user, "name").unwrap();

    assert_eq!(field.name(), "name");
    assert_eq!(field.value().string().as_deref(), Some("alice"));
    assert!(validator::__private::Access::field(&user, "missing").is_none());
}

#[test]
fn required_reports_field_error() {
    let user = User {
        name: String::new(),
    };

    let errors = Validator::new().validate(&user).unwrap_err();
    let fields = errors.into_fields().unwrap();

    assert_eq!(fields.len(), 2);
    assert_eq!(fields[0].namespace().as_str(), "User.name");
    assert_eq!(fields[0].field(), "name");
    assert_eq!(fields[0].rule(), "required");
    assert_eq!(fields[0].reason(), "required");
}

#[test]
fn length_reports_params() {
    let user = User {
        name: "al".to_owned(),
    };

    let errors = Validator::new().validate(&user).unwrap_err();
    let fields = errors.into_fields().unwrap();

    assert_eq!(fields.len(), 1);
    assert_eq!(fields[0].rule(), "length");
    assert_eq!(fields[0].params().get("min"), Some("3"));
    assert_eq!(fields[0].params().get("max"), Some("20"));
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
    let fields = errors.into_fields().unwrap();

    assert_eq!(fields.len(), 1);
    assert_eq!(fields[0].namespace().as_str(), "OptionalUser.email");
    assert_eq!(fields[0].rule(), "required");
}

#[derive(Debug, Validate)]
struct Profile {
    #[validate(required)]
    display_name: String,
}

#[derive(Debug, Validate)]
struct UserWithProfile {
    #[validate(nested)]
    profile: Profile,
}

#[test]
fn nested_struct_remaps_namespace() {
    let user = UserWithProfile {
        profile: Profile {
            display_name: String::new(),
        },
    };

    let fields = Validator::new()
        .validate(&user)
        .unwrap_err()
        .into_fields()
        .unwrap();

    assert_eq!(fields.len(), 1);
    assert_eq!(
        fields[0].namespace().as_str(),
        "UserWithProfile.profile.display_name"
    );
    assert_eq!(fields[0].field(), "display_name");
    assert_eq!(fields[0].rule(), "required");
}

#[derive(Debug, Validate)]
struct OptionalProfileUser {
    #[validate(nested)]
    profile: Option<Profile>,
}

#[test]
fn optional_nested_none_skips_validation() {
    let user = OptionalProfileUser { profile: None };

    Validator::new().validate(&user).unwrap();
}

#[test]
fn optional_nested_some_validates_child() {
    let user = OptionalProfileUser {
        profile: Some(Profile {
            display_name: String::new(),
        }),
    };

    let fields = Validator::new()
        .validate(&user)
        .unwrap_err()
        .into_fields()
        .unwrap();

    assert_eq!(fields.len(), 1);
    assert_eq!(
        fields[0].namespace().as_str(),
        "OptionalProfileUser.profile.display_name"
    );
    assert_eq!(fields[0].rule(), "required");
}

#[test]
fn pure_nested_field_is_not_exposed_as_value_access() {
    let user = UserWithProfile {
        profile: Profile {
            display_name: "alice".to_owned(),
        },
    };

    assert!(validator::__private::Access::field(&user, "profile").is_none());
}

#[derive(Debug, Validate)]
struct RequiredOptionalProfileUser {
    #[validate(required, nested)]
    profile: Option<Profile>,
}

#[test]
fn required_optional_nested_none_fails_required() {
    let user = RequiredOptionalProfileUser { profile: None };

    let fields = Validator::new()
        .validate(&user)
        .unwrap_err()
        .into_fields()
        .unwrap();

    assert_eq!(fields.len(), 1);
    assert_eq!(
        fields[0].namespace().as_str(),
        "RequiredOptionalProfileUser.profile"
    );
    assert_eq!(fields[0].field(), "profile");
    assert_eq!(fields[0].rule(), "required");
}

#[derive(Debug, Validate)]
struct TagForm {
    #[validate(required, gt = 0, dive(required))]
    tags: Vec<String>,
}

#[test]
fn dive_validates_vec_elements_and_reports_index_namespace() {
    let form = TagForm {
        tags: vec!["rust".to_owned(), String::new()],
    };

    let fields = Validator::new()
        .validate(&form)
        .unwrap_err()
        .into_fields()
        .unwrap();

    assert_eq!(fields.len(), 1);
    assert_eq!(fields[0].namespace().as_str(), "TagForm.tags[1]");
    assert_eq!(fields[0].field(), "tags[1]");
    assert_eq!(fields[0].rule(), "required");
}

#[derive(Debug, Validate)]
struct CodeArray {
    #[validate(dive(required))]
    codes: [String; 2],
}

#[test]
fn dive_validates_array_elements() {
    let value = CodeArray {
        codes: ["ok".to_owned(), String::new()],
    };

    let fields = Validator::new()
        .validate(&value)
        .unwrap_err()
        .into_fields()
        .unwrap();

    assert_eq!(fields.len(), 1);
    assert_eq!(fields[0].namespace().as_str(), "CodeArray.codes[1]");
    assert_eq!(fields[0].rule(), "required");
}

#[derive(Debug, Validate)]
struct SliceForm {
    #[validate(dive(required))]
    codes: &'static [&'static str],
}

#[test]
fn dive_validates_slice_reference_elements() {
    static CODES: [&str; 2] = ["ok", ""];
    let value = SliceForm { codes: &CODES };

    let fields = Validator::new()
        .validate(&value)
        .unwrap_err()
        .into_fields()
        .unwrap();

    assert_eq!(fields.len(), 1);
    assert_eq!(fields[0].namespace().as_str(), "SliceForm.codes[1]");
    assert_eq!(fields[0].rule(), "required");
}

#[derive(Debug, Validate)]
struct AddressBook {
    #[validate(dive(nested))]
    addresses: Vec<Profile>,
}

#[test]
fn dive_validates_nested_elements() {
    let value = AddressBook {
        addresses: vec![Profile {
            display_name: String::new(),
        }],
    };

    let fields = Validator::new()
        .validate(&value)
        .unwrap_err()
        .into_fields()
        .unwrap();

    assert_eq!(fields.len(), 1);
    assert_eq!(
        fields[0].namespace().as_str(),
        "AddressBook.addresses[0].display_name"
    );
    assert_eq!(fields[0].rule(), "required");
}

#[derive(Debug, Validate)]
struct LabelMap {
    #[validate(dive(keys(max = 4), values(required)))]
    labels: HashMap<String, String>,
}

#[test]
fn map_dive_validates_hash_map_keys() {
    let value = LabelMap {
        labels: HashMap::from([("toolong".to_owned(), "ok".to_owned())]),
    };

    let fields = Validator::new()
        .validate(&value)
        .unwrap_err()
        .into_fields()
        .unwrap();

    assert_eq!(fields.len(), 1);
    assert_eq!(
        fields[0].namespace().as_str(),
        "LabelMap.labels[\"toolong\"]"
    );
    assert_eq!(fields[0].rule(), "max");
}

#[test]
fn map_dive_validates_hash_map_values() {
    let value = LabelMap {
        labels: HashMap::from([("ok".to_owned(), String::new())]),
    };

    let fields = Validator::new()
        .validate(&value)
        .unwrap_err()
        .into_fields()
        .unwrap();

    assert_eq!(fields.len(), 1);
    assert_eq!(fields[0].namespace().as_str(), "LabelMap.labels[\"ok\"]");
    assert_eq!(fields[0].rule(), "required");
}

#[derive(Debug, Validate)]
struct OrderedLabels {
    #[validate(dive(keys(max = 4), values(required)))]
    labels: BTreeMap<String, String>,
}

#[test]
fn map_dive_validates_btree_map_keys() {
    let value = OrderedLabels {
        labels: BTreeMap::from([("toolong".to_owned(), "ok".to_owned())]),
    };

    let fields = Validator::new()
        .validate(&value)
        .unwrap_err()
        .into_fields()
        .unwrap();

    assert_eq!(fields.len(), 1);
    assert_eq!(
        fields[0].namespace().as_str(),
        "OrderedLabels.labels[\"toolong\"]"
    );
    assert_eq!(fields[0].rule(), "max");
}

#[derive(Debug, Validate)]
struct Inventory {
    #[validate(dive(keys(required), values(nested)))]
    items: HashMap<String, Profile>,
}

#[test]
fn map_dive_validates_nested_values() {
    let value = Inventory {
        items: HashMap::from([(
            "main".to_owned(),
            Profile {
                display_name: String::new(),
            },
        )]),
    };

    let fields = Validator::new()
        .validate(&value)
        .unwrap_err()
        .into_fields()
        .unwrap();

    assert_eq!(fields.len(), 1);
    assert_eq!(
        fields[0].namespace().as_str(),
        "Inventory.items[\"main\"].display_name"
    );
    assert_eq!(fields[0].rule(), "required");
}

#[derive(Debug, Validate)]
#[validate(check = "validate_event")]
struct Event {
    start_at: i64,
    end_at: i64,
}

fn validate_event(event: &Event, valid: &mut validator::valid::Valid<'_>) {
    if event.end_at <= event.start_at {
        valid
            .field("end_at")
            .rule("gt_field")
            .compare("start_at")
            .push();
    }
}

#[test]
fn struct_level_check_reports_compare_param() {
    let event = Event {
        start_at: 10,
        end_at: 5,
    };

    let fields = Validator::new()
        .validate(&event)
        .unwrap_err()
        .into_fields()
        .unwrap();

    assert_eq!(fields.len(), 1);
    assert_eq!(fields[0].namespace().as_str(), "Event.end_at");
    assert_eq!(fields[0].field(), "end_at");
    assert_eq!(fields[0].rule(), "gt_field");
    assert_eq!(fields[0].reason(), "gt_field");
    assert_eq!(fields[0].params().get("compare"), Some("start_at"));
}

#[derive(Debug, Validate)]
struct Signup {
    password: String,

    #[validate(eq_field = "password")]
    confirm_password: String,
}

#[test]
fn eq_field_passes_when_sibling_matches() {
    let signup = Signup {
        password: "secret".to_owned(),
        confirm_password: "secret".to_owned(),
    };

    Validator::new().validate(&signup).unwrap();
}

#[test]
fn eq_field_reports_current_field_error() {
    let signup = Signup {
        password: "secret".to_owned(),
        confirm_password: "different".to_owned(),
    };

    let fields = Validator::new()
        .validate(&signup)
        .unwrap_err()
        .into_fields()
        .unwrap();

    assert_eq!(fields.len(), 1);
    assert_eq!(fields[0].namespace().as_str(), "Signup.confirm_password");
    assert_eq!(fields[0].field(), "confirm_password");
    assert_eq!(fields[0].rule(), "eq_field");
    assert_eq!(fields[0].reason(), "eq_field");
    assert_eq!(fields[0].params().get("compare"), Some("password"));
}

#[derive(Debug, Validate)]
struct EventWindow {
    start_at: i64,

    #[validate(gt_field = "start_at")]
    end_at: i64,
}

#[test]
fn gt_field_compares_sibling_values() {
    Validator::new()
        .validate(&EventWindow {
            start_at: 10,
            end_at: 11,
        })
        .unwrap();

    let fields = Validator::new()
        .validate(&EventWindow {
            start_at: 10,
            end_at: 10,
        })
        .unwrap_err()
        .into_fields()
        .unwrap();

    assert_eq!(fields.len(), 1);
    assert_eq!(fields[0].namespace().as_str(), "EventWindow.end_at");
    assert_eq!(fields[0].rule(), "gt_field");
    assert_eq!(fields[0].params().get("compare"), Some("start_at"));
}

#[derive(Debug, Validate)]
struct OptionalCompare {
    expected: Option<String>,

    #[validate(eq_field = "expected")]
    value: Option<String>,
}

#[test]
fn cross_field_current_none_is_skipped() {
    let value = OptionalCompare {
        expected: Some("secret".to_owned()),
        value: None,
    };

    Validator::new().validate(&value).unwrap();
}

#[test]
fn cross_field_target_none_fails() {
    let value = OptionalCompare {
        expected: None,
        value: Some("secret".to_owned()),
    };

    let fields = Validator::new()
        .validate(&value)
        .unwrap_err()
        .into_fields()
        .unwrap();

    assert_eq!(fields.len(), 1);
    assert_eq!(fields[0].namespace().as_str(), "OptionalCompare.value");
    assert_eq!(fields[0].rule(), "eq_field");
}

#[derive(Debug, Validate)]
struct StrictIntegerKinds {
    left: i32,

    #[validate(eq_field = "left")]
    right: i64,
}

#[test]
fn cross_field_requires_same_concrete_integer_kind() {
    let value = StrictIntegerKinds { left: 7, right: 7 };

    let fields = Validator::new()
        .validate(&value)
        .unwrap_err()
        .into_fields()
        .unwrap();

    assert_eq!(fields.len(), 1);
    assert_eq!(fields[0].rule(), "eq_field");
}

#[derive(Debug, Validate)]
struct StringLengths {
    title: String,

    #[validate(gte_field = "title")]
    summary: String,
}

#[test]
fn ordered_string_cross_field_rules_compare_length() {
    Validator::new()
        .validate(&StringLengths {
            title: "rust".to_owned(),
            summary: "rustacean".to_owned(),
        })
        .unwrap();

    let fields = Validator::new()
        .validate(&StringLengths {
            title: "rustacean".to_owned(),
            summary: "rust".to_owned(),
        })
        .unwrap_err()
        .into_fields()
        .unwrap();

    assert_eq!(fields.len(), 1);
    assert_eq!(fields[0].rule(), "gte_field");
}

#[derive(Debug, Validate)]
#[validate(check = "validate_name_or_title")]
struct Draft {
    name: String,
    title: String,
}

fn validate_name_or_title(draft: &Draft, valid: &mut validator::valid::Valid<'_>) {
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

#[test]
fn struct_level_check_pushes_multiple_param_errors() {
    let draft = Draft {
        name: String::new(),
        title: String::new(),
    };

    let fields = Validator::new()
        .validate(&draft)
        .unwrap_err()
        .into_fields()
        .unwrap();

    assert_eq!(fields.len(), 2);
    assert_eq!(fields[0].namespace().as_str(), "Draft.name");
    assert_eq!(fields[0].rule(), "required_without");
    assert_eq!(fields[0].params().get("field"), Some("title"));
    assert_eq!(fields[1].namespace().as_str(), "Draft.title");
    assert_eq!(fields[1].rule(), "required_without");
    assert_eq!(fields[1].params().get("field"), Some("name"));
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
    let fields = errors.into_fields().unwrap();

    assert_eq!(fields.len(), 1);
    assert_eq!(fields[0].rule(), "username");
    assert_eq!(fields[0].reason(), "length");
    assert_eq!(fields[0].params().get("min"), Some("3"));

    Ok(())
}

#[test]
fn derive_alias_with_unknown_rule_returns_error() -> Result<(), Box<dyn std::error::Error>> {
    let user = AliasUser {
        name: "alice".to_owned(),
    };
    let error = Validator::new()
        .alias("username", "missing_rule")?
        .validate(&user)
        .unwrap_err();

    assert!(matches!(
        error,
        validator::Error::UnknownRule { name } if name == "missing_rule"
    ));

    Ok(())
}

#[test]
fn derive_unknown_alias_returns_error() {
    let user = AliasUser {
        name: "alice".to_owned(),
    };
    let error = Validator::new().validate(&user).unwrap_err();

    assert!(matches!(
        error,
        validator::Error::UnknownAlias { name } if name == "username"
    ));
}

#[derive(Debug, Validate)]
struct SlugPost {
    #[validate(alias = "slug_alias")]
    slug: String,
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
    let fields = errors.into_fields().unwrap();

    assert_eq!(fields.len(), 1);
    assert_eq!(fields[0].rule(), "slug_alias");
    assert_eq!(fields[0].reason(), "slug");

    Ok(())
}

#[test]
fn direct_value_passes() -> Result<(), Box<dyn std::error::Error>> {
    Validator::new().value(&"team@example.com", "required,email")?;

    Ok(())
}

#[test]
fn direct_value_reports_value_namespace() {
    let fields = fields(
        Validator::new()
            .value(&"not-email", "required,email")
            .unwrap_err(),
    );

    assert_eq!(fields.len(), 1);
    assert_eq!(fields[0].namespace().as_str(), "$value");
    assert_eq!(fields[0].field(), "$value");
    assert_eq!(fields[0].rule(), "email");
}

#[test]
fn repeated_direct_value_validation_preserves_errors() {
    let validator = Validator::new();
    validator
        .value(&"team@example.com", "required,email")
        .unwrap();

    let first = fields(validator.value(&"not-email", "required,email").unwrap_err());
    let second = fields(validator.value(&"not-email", "required,email").unwrap_err());

    assert_eq!(first, second);
}

#[test]
fn direct_value_omitempty_skips_empty_value() -> Result<(), Box<dyn std::error::Error>> {
    Validator::new().value(&String::new(), "omitempty,email")?;

    Ok(())
}

#[test]
fn direct_value_alias_preserves_rule_and_reason() -> Result<(), Box<dyn std::error::Error>> {
    let fields = fields(
        Validator::new()
            .alias("username", "required,length(min=3,max=20)")?
            .value(&"al", "username")
            .unwrap_err(),
    );

    assert_eq!(fields.len(), 1);
    assert_eq!(fields[0].rule(), "username");
    assert_eq!(fields[0].reason(), "length");
    assert_eq!(fields[0].params().get("min"), Some("3"));

    Ok(())
}

#[test]
fn direct_value_alias_omitempty_skips_empty_value() -> Result<(), Box<dyn std::error::Error>> {
    Validator::new()
        .alias("optional_email", "omitempty,email")?
        .value(&String::new(), "optional_email")?;

    Ok(())
}

#[test]
fn direct_value_alternatives_preserve_joined_reason() {
    let fields = fields(
        Validator::new()
            .value(&"not-a-color", "hexcolor|rgb|rgba")
            .unwrap_err(),
    );

    assert_eq!(fields.len(), 1);
    assert_eq!(fields[0].rule(), "hexcolor|rgb|rgba");
    assert_eq!(fields[0].reason(), "hexcolor|rgb|rgba");
}

#[test]
fn direct_value_reuses_alias_and_custom_rule() -> Result<(), Box<dyn std::error::Error>> {
    Validator::new()
        .alias("slug_alias", "slug")?
        .rule("slug", Slug)?
        .value(&"hello-rust", "slug_alias")?;

    Ok(())
}

#[test]
fn direct_value_cache_is_generation_scoped() -> Result<(), Box<dyn std::error::Error>> {
    let validator = Validator::new().alias("slug", "required")?;
    assert!(validator.value(&String::new(), "slug").is_err());

    let validator = validator.rule("slug", Slug)?;
    validator.value(&String::new(), "slug")?;

    Ok(())
}

#[test]
fn direct_value_invalid_expression_returns_error() {
    let error = Validator::new().value(&"abc", "length(min=3").unwrap_err();

    assert!(matches!(
        error,
        validator::Error::InvalidRuleExpression { .. }
    ));
}

#[test]
fn direct_value_unknown_rule_returns_error() {
    let error = Validator::new().value(&"abc", "missing_rule").unwrap_err();

    assert!(matches!(
        error,
        validator::Error::UnknownRule { name } if name == "missing_rule"
    ));
}

#[test]
fn direct_value_alias_with_unknown_rule_returns_error() -> Result<(), Box<dyn std::error::Error>> {
    let error = Validator::new()
        .alias("broken_alias", "missing_rule")?
        .value(&"abc", "broken_alias")
        .unwrap_err();

    assert!(matches!(
        error,
        validator::Error::UnknownRule { name } if name == "missing_rule"
    ));

    Ok(())
}

#[test]
fn schema_yaml_validates_map_data() -> Result<(), Box<dyn std::error::Error>> {
    let schema = Schema::from_yaml(
        r#"
fields:
  email:
    type: string
    rules:
      - required
      - email
  title:
    type: string
    rules:
      - required
      - length:
          min: 3
          max: 20
"#,
    )?;
    let data = json!({
        "email": "team@example.com",
        "title": "Validator"
    });

    Validator::with_schema(schema).validate_map(&data)?;

    Ok(())
}

#[test]
fn schema_json_validates_map_data() -> Result<(), Box<dyn std::error::Error>> {
    let schema = Schema::from_json(
        r#"
{
  "fields": {
    "age": {
      "type": "integer",
      "rules": [
        "required",
        { "gte": 0 },
        { "lte": 130 }
      ]
    }
  }
}
"#,
    )?;
    let data = json!({ "age": 42 });

    Validator::with_schema(schema).validate_map(&data)?;

    Ok(())
}

#[test]
fn schema_missing_required_field_reports_required() -> Result<(), Box<dyn std::error::Error>> {
    let schema = Schema::from_yaml(
        r#"
fields:
  email:
    type: string
    rules:
      - required
      - email
"#,
    )?;
    let data = json!({});
    let fields = fields(
        Validator::with_schema(schema)
            .validate_map(&data)
            .unwrap_err(),
    );

    assert_eq!(fields.len(), 1);
    assert_eq!(fields[0].namespace().as_str(), "email");
    assert_eq!(fields[0].rule(), "required");

    Ok(())
}

#[test]
fn schema_missing_optional_field_is_skipped() -> Result<(), Box<dyn std::error::Error>> {
    let schema = Schema::from_yaml(
        r#"
fields:
  email:
    type: string
    rules:
      - email
"#,
    )?;
    let data = json!({});

    Validator::with_schema(schema).validate_map(&data)?;

    Ok(())
}

#[test]
fn schema_nested_object_reports_dotted_namespace() -> Result<(), Box<dyn std::error::Error>> {
    let schema = Schema::from_yaml(
        r#"
fields:
  profile:
    type: object
    fields:
      display_name:
        type: string
        rules:
          - required
"#,
    )?;
    let data = json!({
        "profile": {}
    });
    let fields = fields(
        Validator::with_schema(schema)
            .validate_map(&data)
            .unwrap_err(),
    );

    assert_eq!(fields.len(), 1);
    assert_eq!(fields[0].namespace().as_str(), "profile.display_name");
    assert_eq!(fields[0].field(), "display_name");
    assert_eq!(fields[0].rule(), "required");

    Ok(())
}

#[test]
fn schema_type_mismatch_reports_type_rule() -> Result<(), Box<dyn std::error::Error>> {
    let schema = Schema::from_yaml(
        r#"
fields:
  title:
    type: string
    rules:
      - required
"#,
    )?;
    let data = json!({
        "title": 123
    });
    let fields = fields(
        Validator::with_schema(schema)
            .validate_map(&data)
            .unwrap_err(),
    );

    assert_eq!(fields.len(), 1);
    assert_eq!(fields[0].namespace().as_str(), "title");
    assert_eq!(fields[0].rule(), "type");
    assert_eq!(fields[0].reason(), "type");
    assert_eq!(fields[0].params().get("expected"), Some("string"));

    Ok(())
}

#[test]
fn schema_unknown_rule_returns_config_error() -> Result<(), Box<dyn std::error::Error>> {
    let schema = Schema::from_yaml(
        r#"
fields:
  title:
    type: string
    rules:
      - missing_rule
"#,
    )?;
    let error = Validator::with_schema(schema)
        .validate_map(&json!({ "title": "Rust" }))
        .unwrap_err();

    assert!(matches!(
        error,
        validator::Error::UnknownRule { name } if name == "missing_rule"
    ));

    Ok(())
}

#[test]
fn repeated_schema_validation_preserves_errors() -> Result<(), Box<dyn std::error::Error>> {
    let schema = Schema::from_yaml(
        r#"
fields:
  title:
    type: string
    rules:
      - required
      - length:
          min: 3
"#,
    )?;
    let validator = Validator::with_schema(schema);
    let data = json!({ "title": "rs" });

    let first = fields(validator.validate_map(&data).unwrap_err());
    let second = fields(validator.validate_map(&data).unwrap_err());

    assert_eq!(first, second);

    Ok(())
}

#[test]
fn schema_cache_is_generation_scoped_after_rule_registration()
-> Result<(), Box<dyn std::error::Error>> {
    let schema = Schema::from_yaml(
        r#"
fields:
  slug:
    type: string
    rules:
      - slug
"#,
    )?;
    let validator = Validator::with_schema(schema).alias("slug", "required")?;
    let first = fields(validator.validate_map(&json!({ "slug": "" })).unwrap_err());

    assert_eq!(first.len(), 1);
    assert_eq!(first[0].rule(), "slug");
    assert_eq!(first[0].reason(), "required");

    let validator = validator.rule("slug", Slug)?;
    validator.validate_map(&json!({ "slug": "" }))?;

    let second = fields(
        validator
            .validate_map(&json!({ "slug": "Hello World" }))
            .unwrap_err(),
    );
    assert_eq!(second.len(), 1);
    assert_eq!(second[0].rule(), "slug");
    assert_eq!(second[0].reason(), "slug");

    Ok(())
}

#[test]
fn schema_cache_is_generation_scoped_after_alias_update() -> Result<(), Box<dyn std::error::Error>>
{
    let schema = Schema::from_yaml(
        r#"
fields:
  email:
    type: string
    rules:
      - contact
"#,
    )?;
    let validator = Validator::with_schema(schema).alias("contact", "required,email")?;
    let first = fields(validator.validate_map(&json!({ "email": "" })).unwrap_err());

    assert_eq!(first.len(), 2);
    assert!(first.iter().all(|field| field.rule() == "contact"));
    assert_eq!(first[0].reason(), "required");
    assert_eq!(first[1].reason(), "email");

    let validator = validator.alias("contact", "omitempty,email")?;
    validator.validate_map(&json!({ "email": "" }))?;

    Ok(())
}

#[test]
fn schema_does_not_accept_types_alias() -> Result<(), Box<dyn std::error::Error>> {
    let error = Schema::from_yaml(
        r#"
fields:
  title:
    types: string
    rules:
      - required
"#,
    )
    .unwrap_err();

    assert!(matches!(
        error,
        validator::Error::InvalidSchema { reason }
            if reason.contains("unsupported key 'types'")
    ));

    Ok(())
}

#[test]
fn validate_map_without_schema_returns_error() {
    let error = Validator::new()
        .validate_map(&json!({ "title": "Rust" }))
        .unwrap_err();

    assert!(matches!(error, validator::Error::MissingSchema));
}

#[test]
fn schema_null_required_field_reports_required_only() -> Result<(), Box<dyn std::error::Error>> {
    let schema = Schema::from_yaml(
        r#"
fields:
  email:
    type: string
    rules:
      - required
      - email
"#,
    )?;
    let fields = fields(
        Validator::with_schema(schema)
            .validate_map(&json!({ "email": null }))
            .unwrap_err(),
    );

    assert_eq!(fields.len(), 1);
    assert_eq!(fields[0].namespace().as_str(), "email");
    assert_eq!(fields[0].rule(), "required");

    Ok(())
}

#[test]
fn schema_type_mismatch_skips_rule_group_and_nested_fields()
-> Result<(), Box<dyn std::error::Error>> {
    let schema = Schema::from_yaml(
        r#"
fields:
  profile:
    type: object
    rules:
      - required
    fields:
      email:
        type: string
        rules:
          - required
          - email
"#,
    )?;
    let fields = fields(
        Validator::with_schema(schema)
            .validate_map(&json!({ "profile": "not-object" }))
            .unwrap_err(),
    );

    assert_eq!(fields.len(), 1);
    assert_eq!(fields[0].namespace().as_str(), "profile");
    assert_eq!(fields[0].rule(), "type");

    Ok(())
}

#[derive(Serialize)]
struct SerdeUser {
    #[serde(rename = "user_name")]
    name: String,
    email: String,
}

#[test]
fn validate_serde_validates_serializable_struct() -> Result<(), Box<dyn std::error::Error>> {
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
    let user = SerdeUser {
        name: "alice".to_owned(),
        email: "alice@example.com".to_owned(),
    };

    Validator::with_schema(schema).validate_serde(&user)?;

    Ok(())
}

#[test]
fn validate_serde_uses_serialized_field_names() -> Result<(), Box<dyn std::error::Error>> {
    let schema = Schema::from_yaml(
        r#"
fields:
  user_name:
    type: string
    rules:
      - length:
          min: 3
"#,
    )?;
    let user = SerdeUser {
        name: "al".to_owned(),
        email: "alice@example.com".to_owned(),
    };
    let fields = fields(
        Validator::with_schema(schema)
            .validate_serde(&user)
            .unwrap_err(),
    );

    assert_eq!(fields.len(), 1);
    assert_eq!(fields[0].namespace().as_str(), "user_name");
    assert_eq!(fields[0].struct_namespace().as_str(), "user_name");
    assert_eq!(fields[0].field(), "user_name");
    assert_eq!(fields[0].struct_field(), "user_name");

    Ok(())
}

#[derive(Serialize)]
struct SkippedTitle {
    #[serde(skip_serializing_if = "Option::is_none")]
    title: Option<String>,
}

#[test]
fn validate_serde_skipped_required_field_fails() -> Result<(), Box<dyn std::error::Error>> {
    let schema = Schema::from_yaml(
        r#"
fields:
  title:
    type: string
    rules:
      - required
"#,
    )?;
    let data = SkippedTitle { title: None };
    let fields = fields(
        Validator::with_schema(schema)
            .validate_serde(&data)
            .unwrap_err(),
    );

    assert_eq!(fields.len(), 1);
    assert_eq!(fields[0].namespace().as_str(), "title");
    assert_eq!(fields[0].rule(), "required");

    Ok(())
}

#[derive(Serialize)]
struct SerdeProfile {
    email: String,
}

#[derive(Serialize)]
struct FlattenedUser {
    name: String,
    #[serde(flatten)]
    profile: SerdeProfile,
}

#[test]
fn validate_serde_validates_flattened_fields() -> Result<(), Box<dyn std::error::Error>> {
    let schema = Schema::from_yaml(
        r#"
fields:
  name:
    type: string
    rules:
      - required
  email:
    type: string
    rules:
      - email
"#,
    )?;
    let user = FlattenedUser {
        name: "alice".to_owned(),
        profile: SerdeProfile {
            email: "not-email".to_owned(),
        },
    };
    let fields = fields(
        Validator::with_schema(schema)
            .validate_serde(&user)
            .unwrap_err(),
    );

    assert_eq!(fields.len(), 1);
    assert_eq!(fields[0].namespace().as_str(), "email");
    assert_eq!(fields[0].rule(), "email");

    Ok(())
}

#[test]
fn validate_serde_non_object_root_reports_type_error() -> Result<(), Box<dyn std::error::Error>> {
    let schema = Schema::from_yaml(
        r#"
fields:
  title:
    type: string
"#,
    )?;
    let fields = fields(
        Validator::with_schema(schema)
            .validate_serde("not-object")
            .unwrap_err(),
    );

    assert_eq!(fields.len(), 1);
    assert_eq!(fields[0].namespace().as_str(), "$value");
    assert_eq!(fields[0].rule(), "type");
    assert_eq!(fields[0].params().get("expected"), Some("object"));

    Ok(())
}

struct BrokenSerde;

impl serde::Serialize for BrokenSerde {
    fn serialize<S>(&self, _serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        Err(serde::ser::Error::custom("broken serde value"))
    }
}

#[test]
fn validate_serde_reports_serialization_error() -> Result<(), Box<dyn std::error::Error>> {
    let schema = Schema::from_yaml(
        r#"
fields:
  title:
    type: string
"#,
    )?;
    let error = Validator::with_schema(schema)
        .validate_serde(&BrokenSerde)
        .unwrap_err();

    assert!(matches!(
        error,
        validator::Error::InvalidData { reason } if reason.contains("broken serde value")
    ));

    Ok(())
}

#[test]
fn validate_serde_without_schema_returns_error() {
    let error = Validator::new()
        .validate_serde(&json!({ "title": "Rust" }))
        .unwrap_err();

    assert!(matches!(error, validator::Error::MissingSchema));
}

#[test]
fn direct_value_uses_new_common_rules() {
    Validator::new()
        .value(&"https://example.com", "required,http_url,https_url")
        .unwrap();
    Validator::new()
        .value(&"urn:isbn:0451450523", "uri")
        .unwrap();
    Validator::new().value(&"127.0.0.1", "ip,ipv4").unwrap();
    Validator::new()
        .value(&"192.168.0.0/24", "cidr,cidrv4")
        .unwrap();
    Validator::new()
        .value(&"2001:db8::/32", "cidr,cidrv6")
        .unwrap();
    Validator::new()
        .value(&"api.example.com", "hostname,fqdn")
        .unwrap();
    Validator::new()
        .value(&"1.foo.com", "hostname_rfc1123")
        .unwrap();
    Validator::new().value(&"443", "port").unwrap();
    Validator::new()
        .value(&"a987fbc9-4bed-3078-cf07-9141ba07c9f3", "uuid")
        .unwrap();
    Validator::new()
        .value(&"550e8400-e29b-41d4-a716-446655440000", "uuid4")
        .unwrap();
    Validator::new()
        .value(&"01bx5zzkbkactav9wevgemmvrz", "ulid")
        .unwrap();
    Validator::new().value(&r#"{"ok":true}"#, "json").unwrap();
    Validator::new()
        .value(&"2026-07-08T12:30:00+08:00", "datetime")
        .unwrap();
    Validator::new()
        .value(&42_u32, "eq(value=42),ne(value=0)")
        .unwrap();
    Validator::new()
        .value(
            &"hello!",
            r#"ascii,containsany(value="!@#?"),noneof("root","admin")"#,
        )
        .unwrap();
    Validator::new().value(&2_u8, "oneof(1,2,3)").unwrap();
    Validator::new().value(&4_i32, "noneof(1,2,3)").unwrap();
}

#[test]
fn direct_value_reports_new_string_choice_rules() {
    let fields = fields(
        Validator::new()
            .value(
                &"root",
                r#"containsany(value="!@#?"),noneof("root","admin")"#,
            )
            .unwrap_err(),
    );

    assert_eq!(fields.len(), 2);
    assert_eq!(fields[0].rule(), "containsany");
    assert_eq!(fields[0].params().get("value"), Some("!@#?"));
    assert_eq!(fields[1].rule(), "noneof");
    assert_eq!(fields[1].params().get("values"), Some("root,admin"));
}

#[test]
fn schema_uses_new_common_rules() -> Result<(), Box<dyn std::error::Error>> {
    let schema = Schema::from_yaml(
        r#"
fields:
  state:
    type: string
    rules:
      - eq:
          value: published
  source_url:
    type: string
    rules:
      - https_url
  canonical_uri:
    type: string
    rules:
      - uri
  request_ip:
    type: string
    rules:
      - ip
  network:
    type: string
    rules:
      - cidr
  host:
    type: string
    rules:
      - fqdn
  rfc_host:
    type: string
    rules:
      - hostname_rfc1123
  port:
    type: string
    rules:
      - port
  id:
    type: string
    rules:
      - uuid
  request_id:
    type: string
    rules:
      - uuid4
  public_id:
    type: string
    rules:
      - ulid
  labels:
    type: object
    rules:
      - unique
  metadata:
    type: string
    rules:
      - json
  created_at:
    type: string
    rules:
      - datetime
  code:
    type: string
    rules:
      - ascii
      - containsany:
          value: "-"
  username:
    type: string
    rules:
      - noneof(root,admin)
  priority:
    type: uint
    rules:
      - oneof(1,2,3)
  tags:
    type: array
    rules:
      - unique
"#,
    )?;
    let data = json!({
        "state": "draft",
        "source_url": "http://example.com",
        "canonical_uri": "not uri",
        "request_ip": "not-ip",
        "network": "10.0.0.0/33",
        "host": "api",
        "rfc_host": "foo.bar:80",
        "port": "0",
        "id": "A987FBC9-4BED-3078-CF07-9141BA07C9F3",
        "request_id": "a987fbc9-4bed-3078-cf07-9141ba07c9f3",
        "public_id": "01BX5ZZKBKACTAV9WEVGEMMVRU",
        "labels": {"a": "rust", "b": "rust"},
        "metadata": "{not-json}",
        "created_at": "2026-02-30T12:00:00Z",
        "code": "你好",
        "username": "root",
        "priority": 4,
        "tags": ["rust", "rust"]
    });
    let fields = fields(
        Validator::with_schema(schema)
            .validate_map(&data)
            .unwrap_err(),
    );
    let failures = fields
        .iter()
        .map(|field| (field.namespace().as_str(), field.rule()))
        .collect::<Vec<_>>();

    assert_eq!(
        failures,
        vec![
            ("canonical_uri", "uri"),
            ("code", "ascii"),
            ("code", "containsany"),
            ("created_at", "datetime"),
            ("host", "fqdn"),
            ("id", "uuid"),
            ("labels", "unique"),
            ("metadata", "json"),
            ("network", "cidr"),
            ("port", "port"),
            ("priority", "oneof"),
            ("public_id", "ulid"),
            ("request_id", "uuid4"),
            ("request_ip", "ip"),
            ("rfc_host", "hostname_rfc1123"),
            ("source_url", "https_url"),
            ("state", "eq"),
            ("tags", "unique"),
            ("username", "noneof"),
        ]
    );
    assert_eq!(fields[2].params().get("value"), Some("-"));
    assert_eq!(fields[10].params().get("values"), Some("1,2,3"));
    assert_eq!(fields[16].params().get("value"), Some("published"));
    assert_eq!(fields[18].params().get("values"), Some("root,admin"));

    Ok(())
}

#[test]
fn schema_cross_field_rules_compare_sibling_fields() -> Result<(), Box<dyn std::error::Error>> {
    let schema = Schema::from_yaml(
        r#"
fields:
  password:
    type: string
  confirm_password:
    type: string
    rules:
      - eq_field: password
  start_at:
    type: integer
  end_at:
    type: integer
    rules:
      - gt_field: start_at
"#,
    )?;
    let fields = fields(
        Validator::with_schema(schema)
            .validate_map(&json!({
                "password": "secret",
                "confirm_password": "different",
                "start_at": 10,
                "end_at": 10
            }))
            .unwrap_err(),
    );

    assert_eq!(fields.len(), 2);
    assert_eq!(fields[0].namespace().as_str(), "confirm_password");
    assert_eq!(fields[0].rule(), "eq_field");
    assert_eq!(fields[0].params().get("compare"), Some("password"));
    assert_eq!(fields[1].namespace().as_str(), "end_at");
    assert_eq!(fields[1].rule(), "gt_field");
    assert_eq!(fields[1].params().get("compare"), Some("start_at"));

    Ok(())
}

#[test]
fn schema_cross_field_rule_passes() -> Result<(), Box<dyn std::error::Error>> {
    let schema = Schema::from_yaml(
        r#"
fields:
  start_at:
    type: integer
  end_at:
    type: integer
    rules:
      - gt_field: start_at
"#,
    )?;
    let data = json!({
        "start_at": 10,
        "end_at": 11
    });

    Validator::with_schema(schema).validate_map(&data)?;

    Ok(())
}

#[test]
fn schema_cross_field_undeclared_target_returns_config_error()
-> Result<(), Box<dyn std::error::Error>> {
    let schema = Schema::from_yaml(
        r#"
fields:
  confirm_password:
    type: string
    rules:
      - eq_field: password
"#,
    )?;
    let error = Validator::with_schema(schema)
        .validate_map(&json!({
            "confirm_password": "secret"
        }))
        .unwrap_err();

    assert!(matches!(
        error,
        validator::Error::InvalidSchema { reason }
            if reason.contains("references undeclared field 'password'")
    ));

    Ok(())
}

#[test]
fn schema_cross_field_missing_target_value_fails_validation()
-> Result<(), Box<dyn std::error::Error>> {
    let schema = Schema::from_yaml(
        r#"
fields:
  password:
    type: string
  confirm_password:
    type: string
    rules:
      - eq_field: password
"#,
    )?;
    let fields = fields(
        Validator::with_schema(schema)
            .validate_map(&json!({
                "confirm_password": "secret"
            }))
            .unwrap_err(),
    );

    assert_eq!(fields.len(), 1);
    assert_eq!(fields[0].namespace().as_str(), "confirm_password");
    assert_eq!(fields[0].rule(), "eq_field");
    assert_eq!(fields[0].params().get("compare"), Some("password"));

    Ok(())
}

#[derive(Debug, Validate)]
struct TimeEvent {
    #[validate(lte)]
    created_at: SystemTime,

    #[validate(gt)]
    expires_at: SystemTime,
}

#[test]
fn system_time_compares_with_captured_now() {
    let now = SystemTime::now();

    Validator::new()
        .validate(&TimeEvent {
            created_at: now
                .checked_sub(Duration::from_secs(60))
                .expect("current time should support subtracting one minute"),
            expires_at: now + Duration::from_secs(60),
        })
        .unwrap();

    let fields = fields(
        Validator::new()
            .validate(&TimeEvent {
                created_at: now + Duration::from_secs(60),
                expires_at: now
                    .checked_sub(Duration::from_secs(60))
                    .expect("current time should support subtracting one minute"),
            })
            .unwrap_err(),
    );
    let rules = fields.iter().map(|field| field.rule()).collect::<Vec<_>>();

    assert_eq!(rules, vec!["lte", "gt"]);
    assert_eq!(fields[0].kind(), Kind::Time);
    assert_eq!(fields[1].kind(), Kind::Time);
}

#[test]
fn direct_system_time_value_uses_now_rules() -> Result<(), Box<dyn std::error::Error>> {
    let now = SystemTime::now();
    let past = now
        .checked_sub(Duration::from_secs(60))
        .expect("current time should support subtracting one minute");
    let future = now + Duration::from_secs(60);

    Validator::new().value(&past, "lte")?;
    Validator::new().value(&future, "gt")?;

    Ok(())
}

#[derive(Debug, Validate)]
struct TimeWindow {
    created_at: SystemTime,

    #[validate(gt_field = "created_at")]
    updated_at: SystemTime,

    #[validate(eq_field = "created_at")]
    copied_at: SystemTime,
}

#[test]
fn system_time_cross_field_rules_compare_time_values() {
    let created_at = SystemTime::now();

    Validator::new()
        .validate(&TimeWindow {
            created_at,
            updated_at: created_at + Duration::from_secs(60),
            copied_at: created_at,
        })
        .unwrap();

    let fields = fields(
        Validator::new()
            .validate(&TimeWindow {
                created_at,
                updated_at: created_at,
                copied_at: created_at + Duration::from_secs(1),
            })
            .unwrap_err(),
    );
    let rules = fields.iter().map(|field| field.rule()).collect::<Vec<_>>();

    assert_eq!(rules, vec!["gt_field", "eq_field"]);
}

#[derive(Debug, Validate)]
struct TimeStrictKind {
    created_at_text: String,

    #[validate(gt_field = "created_at_text")]
    updated_at: SystemTime,
}

#[test]
fn system_time_cross_field_requires_time_kind() {
    let fields = fields(
        Validator::new()
            .validate(&TimeStrictKind {
                created_at_text: "2026-07-08T00:00:00Z".to_owned(),
                updated_at: SystemTime::now(),
            })
            .unwrap_err(),
    );

    assert_eq!(fields.len(), 1);
    assert_eq!(fields[0].rule(), "gt_field");
}

#[derive(Debug, Validate)]
struct ParameterizedTime {
    #[validate(gt = "2026-07-08T00:00:00Z")]
    at: SystemTime,
}

#[test]
fn parameterized_system_time_comparison_returns_config_error() {
    let error = Validator::new()
        .validate(&ParameterizedTime {
            at: SystemTime::now(),
        })
        .unwrap_err();

    assert!(matches!(
        error,
        Error::InvalidRuleExpression { reason, .. }
            if reason.contains("SystemTime comparison does not support literal parameters")
    ));
}

#[derive(Debug, Validate)]
struct EqualNowTime {
    #[validate(eq)]
    at: SystemTime,
}

#[test]
fn system_time_eq_now_returns_config_error() {
    let error = Validator::new()
        .validate(&EqualNowTime {
            at: SystemTime::now(),
        })
        .unwrap_err();

    assert!(matches!(
        error,
        Error::InvalidRuleExpression { reason, .. }
            if reason.contains("SystemTime eq/ne against the current time is unsupported")
    ));
}

#[test]
fn direct_system_time_parameter_returns_config_error() {
    let error = Validator::new()
        .value(&SystemTime::now(), r#"gt(value="2026-07-08T00:00:00Z")"#)
        .unwrap_err();

    assert!(matches!(
        error,
        Error::InvalidRuleExpression { reason, .. }
            if reason.contains("SystemTime comparison does not support literal parameters")
    ));
}

#[test]
fn direct_alternative_system_time_config_error_is_not_swallowed() {
    let error = Validator::new()
        .value(
            &SystemTime::now(),
            r#"gt(value="2026-07-08T00:00:00Z")|lte"#,
        )
        .unwrap_err();

    assert!(matches!(
        error,
        Error::InvalidRuleExpression { reason, .. }
            if reason.contains("SystemTime comparison does not support literal parameters")
    ));
}

#[test]
fn schema_rejects_native_time_type() {
    let error = Schema::from_yaml(
        r#"
fields:
  created_at:
    type: time
"#,
    )
    .unwrap_err();

    assert!(matches!(
        error,
        Error::InvalidSchema { reason } if reason.contains("unsupported type 'time'")
    ));
}
