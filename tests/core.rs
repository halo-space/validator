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

fn param_list<'a>(field: &'a validator::FieldError, name: &str) -> Vec<&'a str> {
    field
        .params()
        .list(name)
        .expect("expected list parameter")
        .iter()
        .map(String::as_str)
        .collect()
}

fn param_pairs<'a>(field: &'a validator::FieldError, name: &str) -> Vec<(&'a str, &'a str)> {
    field
        .params()
        .pairs(name)
        .expect("expected pair parameter")
        .iter()
        .map(|(name, value)| (name.as_str(), value.as_str()))
        .collect()
}

fn param_pair<'a>(field: &'a validator::FieldError, name: &str) -> Option<&'a str> {
    field
        .params()
        .pairs("conditions")?
        .iter()
        .find_map(|(field, value)| (field == name).then_some(value.as_str()))
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
    assert_eq!(fields[0].params().text("min"), Some("3"));
    assert_eq!(fields[0].params().text("max"), Some("20"));
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
    assert_eq!(fields[0].kind(), Kind::Option);
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

#[test]
fn map_dive_escapes_namespace_keys() {
    let value = LabelMap {
        labels: HashMap::from([("quoted\"key\\line\n".to_owned(), String::new())]),
    };

    let fields = Validator::new()
        .validate(&value)
        .unwrap_err()
        .into_fields()
        .unwrap();

    assert_eq!(
        fields[0].namespace().as_str(),
        "LabelMap.labels[\"quoted\\\"key\\\\line\\n\"]"
    );
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
    assert_eq!(fields[0].kind(), Kind::Int(IntKind::I64));
    assert_eq!(fields[0].params().text("compare"), Some("start_at"));
}

#[derive(Debug, Validate)]
#[validate(check = "validate_typed_fields")]
struct TypedFields {
    name: String,
    score: i32,
    created_at: SystemTime,
    nickname: Option<String>,
}

fn validate_typed_fields(value: &TypedFields, valid: &mut validator::valid::Valid<'_>) {
    let _ = (&value.name, value.score, value.created_at);
    for field in ["name", "score", "created_at", "nickname"] {
        valid.field(field).rule("typed").push();
    }
}

#[test]
fn struct_level_errors_keep_derived_field_kinds() {
    let fields = Validator::new()
        .validate(&TypedFields {
            name: String::new(),
            score: 0,
            created_at: SystemTime::UNIX_EPOCH,
            nickname: None,
        })
        .unwrap_err()
        .into_fields()
        .unwrap();

    assert_eq!(fields[0].kind(), Kind::String);
    assert_eq!(fields[1].kind(), Kind::Int(IntKind::I32));
    assert_eq!(fields[2].kind(), Kind::Time);
    assert_eq!(fields[3].kind(), Kind::Option);
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
    assert_eq!(fields[0].params().text("compare"), Some("password"));
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
    assert_eq!(fields[0].params().text("compare"), Some("start_at"));
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
            title: "zz".to_owned(),
            summary: "aaa".to_owned(),
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
struct BoolOrdering {
    lower: bool,

    #[validate(gt_field = "lower")]
    higher: bool,
}

#[test]
fn ordered_cross_field_rules_do_not_order_bool_values() {
    let fields = Validator::new()
        .validate(&BoolOrdering {
            lower: false,
            higher: true,
        })
        .unwrap_err()
        .into_fields()
        .unwrap();

    assert_eq!(fields.len(), 1);
    assert_eq!(fields[0].rule(), "gt_field");
}

#[derive(Debug, Validate)]
struct FloatRelations {
    source: f64,

    #[validate(ne_field = "source")]
    different: f64,

    #[validate(gt_field = "source")]
    ordered: f64,
}

#[test]
fn cross_field_float_nan_is_unequal_and_unordered() {
    let fields = Validator::new()
        .validate(&FloatRelations {
            source: f64::NAN,
            different: f64::NAN,
            ordered: f64::NAN,
        })
        .unwrap_err()
        .into_fields()
        .unwrap();

    assert_eq!(fields.len(), 1);
    assert_eq!(fields[0].rule(), "gt_field");
}

#[derive(Debug, Validate)]
struct RawIdentifierFields {
    #[validate(required)]
    r#type: String,

    #[validate(eq_field = "type")]
    copy: String,
}

#[test]
fn derive_canonicalizes_raw_field_metadata_and_targets() {
    let fields = Validator::new()
        .validate(&RawIdentifierFields {
            r#type: String::new(),
            copy: "value".to_owned(),
        })
        .unwrap_err()
        .into_fields()
        .unwrap();

    assert_eq!(fields.len(), 2);
    assert_eq!(fields[0].namespace().as_str(), "RawIdentifierFields.type");
    assert_eq!(fields[0].field(), "type");
    assert_eq!(fields[1].params().text("compare"), Some("type"));
}

#[derive(Debug, Validate)]
struct FieldText {
    needle: String,
    forbidden: String,

    #[validate(fieldcontains = "needle", fieldexcludes = "forbidden")]
    body: String,
}

#[test]
fn field_string_rules_compare_sibling_fields() {
    Validator::new()
        .validate(&FieldText {
            needle: "rust".to_owned(),
            forbidden: "go".to_owned(),
            body: "hello rust".to_owned(),
        })
        .unwrap();

    let fields = Validator::new()
        .validate(&FieldText {
            needle: "rust".to_owned(),
            forbidden: "go".to_owned(),
            body: "hello go".to_owned(),
        })
        .unwrap_err()
        .into_fields()
        .unwrap();
    let rules = fields.iter().map(|field| field.rule()).collect::<Vec<_>>();

    assert_eq!(rules, vec!["fieldcontains", "fieldexcludes"]);
    assert_eq!(fields[0].params().text("compare"), Some("needle"));
    assert_eq!(fields[1].params().text("compare"), Some("forbidden"));
}

#[derive(Debug, Validate)]
struct ConditionalFieldRules {
    status: String,
    mode: String,
    email: String,
    phone: Option<String>,
    backup_email: String,
    backup_phone: Option<String>,

    #[validate(required_if(status = "published"))]
    published_at: Option<String>,

    #[validate(required_unless(status = "draft"))]
    title: String,

    #[validate(skip_unless(status = "archived"))]
    reviewer: String,

    #[validate(required_with("email", "phone"))]
    contact_name: String,

    #[validate(required_without("email", "phone"))]
    fallback_contact: String,

    #[validate(required_with_all("email", "phone"))]
    all_contact_name: String,

    #[validate(required_without_all("email", "phone"))]
    all_fallback_contact: String,

    #[validate(excluded_if(status = "archived"))]
    archive_note: String,

    #[validate(excluded_unless(status = "draft"))]
    draft_only_note: String,

    #[validate(excluded_with("backup_email", "backup_phone"))]
    backup_override: String,

    #[validate(excluded_with_all("backup_email", "backup_phone"))]
    backup_all_override: String,

    #[validate(excluded_without("backup_email", "backup_phone"))]
    backup_missing_override: String,

    #[validate(excluded_without_all("backup_email", "backup_phone"))]
    backup_all_missing_override: String,

    #[validate(excluded_if(mode = "private"))]
    private_note: Option<String>,
}

#[test]
fn conditional_field_rules_validate_sibling_fields() {
    Validator::new()
        .validate(&ConditionalFieldRules {
            status: "draft".to_owned(),
            mode: "public".to_owned(),
            email: String::new(),
            phone: None,
            backup_email: "backup@example.com".to_owned(),
            backup_phone: Some("123".to_owned()),
            published_at: None,
            title: String::new(),
            reviewer: String::new(),
            contact_name: String::new(),
            fallback_contact: "support".to_owned(),
            all_contact_name: String::new(),
            all_fallback_contact: "support".to_owned(),
            archive_note: String::new(),
            draft_only_note: "draft note".to_owned(),
            backup_override: String::new(),
            backup_all_override: String::new(),
            backup_missing_override: String::new(),
            backup_all_missing_override: String::new(),
            private_note: None,
        })
        .unwrap();

    let fields = Validator::new()
        .validate(&ConditionalFieldRules {
            status: "archived".to_owned(),
            mode: "private".to_owned(),
            email: "editor@example.com".to_owned(),
            phone: None,
            backup_email: String::new(),
            backup_phone: None,
            published_at: None,
            title: String::new(),
            reviewer: String::new(),
            contact_name: String::new(),
            fallback_contact: String::new(),
            all_contact_name: String::new(),
            all_fallback_contact: String::new(),
            archive_note: "archived".to_owned(),
            draft_only_note: "draft only".to_owned(),
            backup_override: "override".to_owned(),
            backup_all_override: String::new(),
            backup_missing_override: "missing".to_owned(),
            backup_all_missing_override: "all missing".to_owned(),
            private_note: Some("secret".to_owned()),
        })
        .unwrap_err()
        .into_fields()
        .unwrap();
    let rules = fields.iter().map(|field| field.rule()).collect::<Vec<_>>();

    assert_eq!(
        rules,
        vec![
            "required_unless",
            "skip_unless",
            "required_with",
            "required_without",
            "excluded_if",
            "excluded_unless",
            "excluded_without",
            "excluded_without_all",
            "excluded_if"
        ]
    );
    assert_eq!(param_pair(&fields[0], "status"), Some("draft"));
    assert_eq!(param_pair(&fields[1], "status"), Some("archived"));
    assert_eq!(param_list(&fields[2], "fields"), vec!["email", "phone"]);
    assert_eq!(param_list(&fields[3], "fields"), vec!["email", "phone"]);
    assert_eq!(param_pair(&fields[4], "status"), Some("archived"));
    assert_eq!(param_pair(&fields[5], "status"), Some("draft"));
    assert_eq!(
        param_list(&fields[6], "fields"),
        vec!["backup_email", "backup_phone"]
    );
    assert_eq!(
        param_list(&fields[7], "fields"),
        vec!["backup_email", "backup_phone"]
    );
    assert_eq!(param_pair(&fields[8], "mode"), Some("private"));
}

#[derive(Debug, Validate)]
struct ConditionalAllFieldRules {
    email: String,
    phone: Option<String>,
    backup_email: String,
    backup_phone: Option<String>,

    #[validate(required_with_all("email", "phone"))]
    contact_name: String,

    #[validate(required_without_all("backup_email", "backup_phone"))]
    fallback_contact: String,

    #[validate(excluded_with("email", "phone"))]
    contact_override: String,

    #[validate(excluded_with_all("email", "phone"))]
    all_contact_override: String,

    #[validate(excluded_without("backup_email", "backup_phone"))]
    missing_backup_override: String,

    #[validate(excluded_without_all("backup_email", "backup_phone"))]
    all_missing_backup_override: String,
}

#[test]
fn conditional_all_rules_distinguish_any_and_all_fields() {
    Validator::new()
        .validate(&ConditionalAllFieldRules {
            email: "editor@example.com".to_owned(),
            phone: None,
            backup_email: "backup@example.com".to_owned(),
            backup_phone: None,
            contact_name: String::new(),
            fallback_contact: String::new(),
            contact_override: String::new(),
            all_contact_override: String::new(),
            missing_backup_override: String::new(),
            all_missing_backup_override: String::new(),
        })
        .unwrap();

    let fields = Validator::new()
        .validate(&ConditionalAllFieldRules {
            email: "editor@example.com".to_owned(),
            phone: Some("123".to_owned()),
            backup_email: String::new(),
            backup_phone: None,
            contact_name: String::new(),
            fallback_contact: String::new(),
            contact_override: "override".to_owned(),
            all_contact_override: "all override".to_owned(),
            missing_backup_override: "missing".to_owned(),
            all_missing_backup_override: "all missing".to_owned(),
        })
        .unwrap_err()
        .into_fields()
        .unwrap();
    let rules = fields.iter().map(|field| field.rule()).collect::<Vec<_>>();

    assert_eq!(
        rules,
        vec![
            "required_with_all",
            "required_without_all",
            "excluded_with",
            "excluded_with_all",
            "excluded_without",
            "excluded_without_all"
        ]
    );
    for field in fields {
        let expected = match field.rule() {
            "required_with_all" | "excluded_with" | "excluded_with_all" => {
                vec!["email", "phone"]
            }
            "required_without_all" | "excluded_without" | "excluded_without_all" => {
                vec!["backup_email", "backup_phone"]
            }
            rule => panic!("unexpected rule {rule}"),
        };

        assert_eq!(param_list(&field, "fields"), expected);
    }
}

#[derive(Debug, Validate)]
struct NumericConditionalFieldRule {
    level: u8,

    #[validate(required_if(level = 3))]
    badge: String,
}

#[test]
fn conditional_pair_rule_compares_typed_values() {
    Validator::new()
        .validate(&NumericConditionalFieldRule {
            level: 2,
            badge: String::new(),
        })
        .unwrap();

    let fields = Validator::new()
        .validate(&NumericConditionalFieldRule {
            level: 3,
            badge: String::new(),
        })
        .unwrap_err()
        .into_fields()
        .unwrap();

    assert_eq!(fields.len(), 1);
    assert_eq!(fields[0].rule(), "required_if");
    assert_eq!(param_pairs(&fields[0], "conditions"), vec![("level", "3")]);
}

#[derive(Debug, Validate)]
struct NullableConditionalFieldRule {
    level: Option<u8>,

    #[validate(required_if(level = "null"))]
    badge: String,
}

#[test]
fn conditional_pair_rule_accepts_null_for_typed_fields() {
    let fields = Validator::new()
        .validate(&NullableConditionalFieldRule {
            level: None,
            badge: String::new(),
        })
        .unwrap_err()
        .into_fields()
        .unwrap();
    assert_eq!(fields[0].rule(), "required_if");

    Validator::new()
        .validate(&NullableConditionalFieldRule {
            level: Some(1),
            badge: String::new(),
        })
        .unwrap();

    let schema = Schema::from_yaml(
        r#"
fields:
  level:
    type: uint
  badge:
    type: string
    rules:
      - required_if:
          level: "null"
"#,
    )
    .unwrap();
    let validator = Validator::with_schema(schema);
    let fields = validator
        .validate_map(&json!({ "badge": "" }))
        .unwrap_err()
        .into_fields()
        .unwrap();
    assert_eq!(fields[0].rule(), "required_if");

    validator
        .validate_map(&json!({ "level": 1, "badge": "" }))
        .unwrap();
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
    assert_eq!(fields[0].kind(), Kind::String);
    assert_eq!(fields[0].params().text("field"), Some("title"));
    assert_eq!(fields[1].namespace().as_str(), "Draft.title");
    assert_eq!(fields[1].rule(), "required_without");
    assert_eq!(fields[1].kind(), Kind::String);
    assert_eq!(fields[1].params().text("field"), Some("name"));
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
    assert_eq!(fields[0].params().text("min"), Some("3"));

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
        validator::Error::UnknownRule { name } if name == "username"
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
    assert_eq!(fields[0].params().text("min"), Some("3"));

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
fn direct_value_alias_omitempty_skips_following_rules() -> Result<(), Box<dyn std::error::Error>> {
    Validator::new()
        .alias("optional_email", "omitempty,email")?
        .value(&String::new(), "optional_email,required")?;

    Ok(())
}

#[test]
fn alias_omitempty_in_alternative_skips_following_rules() -> Result<(), Box<dyn std::error::Error>>
{
    Validator::new()
        .alias("optional_email", "omitempty,email")?
        .value(&String::new(), "email|optional_email,required")?;

    Ok(())
}

#[derive(Debug, Validate)]
struct OptionalAliasEmail {
    #[validate(alias = "optional_email", required)]
    email: String,
}

#[test]
fn derive_alias_omitempty_skips_following_rules() -> Result<(), Box<dyn std::error::Error>> {
    Validator::new()
        .alias("optional_email", "omitempty,email")?
        .validate(&OptionalAliasEmail {
            email: String::new(),
        })?;

    Ok(())
}

#[test]
fn schema_alias_omitempty_skips_following_rules() -> Result<(), Box<dyn std::error::Error>> {
    let schema = Schema::from_yaml(
        r#"
fields:
  email:
    type: string
    rules:
      - optional_email
      - required
"#,
    )?;

    Validator::with_schema(schema)
        .alias("optional_email", "omitempty,email")?
        .validate_map(&serde_json::json!({ "email": "" }))?;

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
fn direct_value_rejects_rule_alias_name_collision() -> Result<(), Box<dyn std::error::Error>> {
    let validator = Validator::new().alias("slug", "required")?;
    assert!(validator.value(&String::new(), "slug").is_err());

    let Err(error) = validator.rule("slug", Slug) else {
        panic!("expected duplicate name error");
    };
    assert!(matches!(error, Error::DuplicateName { name } if name == "slug"));

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
      "type": "int",
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
    assert_eq!(fields[0].params().text("expected"), Some("string"));

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
fn schema_rejects_alias_rule_name_collision() -> Result<(), Box<dyn std::error::Error>> {
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

    let Err(error) = validator.rule("slug", Slug) else {
        panic!("expected duplicate name error");
    };
    assert!(matches!(error, Error::DuplicateName { name } if name == "slug"));

    Ok(())
}

#[test]
fn schema_rejects_duplicate_alias_name() -> Result<(), Box<dyn std::error::Error>> {
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

    let Err(error) = validator.alias("contact", "omitempty,email") else {
        panic!("expected duplicate name error");
    };
    assert!(matches!(error, Error::DuplicateName { name } if name == "contact"));

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
fn schema_rejects_compatibility_type_names() {
    for ty in ["bool", "integer", "number", "map"] {
        let error = Schema::from_yaml(format!("fields:\n  value:\n    type: {ty}\n")).unwrap_err();

        assert!(matches!(
            error,
            Error::InvalidSchema { reason }
                if reason.contains(&format!("unsupported type '{ty}'"))
        ));
    }
}

#[test]
fn schema_rejects_unknown_field_key() {
    let error = Schema::from_yaml(
        r#"
fields:
  title:
    type: string
    rulse:
      - required
"#,
    )
    .unwrap_err();

    assert!(matches!(
        error,
        Error::InvalidSchema { reason }
            if reason.contains("field 'title'") && reason.contains("unknown key 'rulse'")
    ));
}

#[test]
fn schema_rejects_unknown_top_level_key() {
    let error = Schema::from_json(
        r#"{
  "fields": {},
  "version": 1
}"#,
    )
    .unwrap_err();

    assert!(matches!(
        error,
        Error::InvalidSchema { reason } if reason.contains("unknown key 'version'")
    ));
}

#[test]
fn schema_preserves_explicit_empty_fields_structure() {
    let scalar = Schema::from_yaml(
        r#"
fields:
  title:
    type: string
    fields: {}
"#,
    )
    .unwrap_err();
    assert!(matches!(scalar, Error::InvalidSchema { .. }));

    let inferred = Schema::from_yaml(
        r#"
fields:
  metadata:
    fields: {}
"#,
    )
    .unwrap();
    let fields = fields(
        Validator::with_schema(inferred)
            .validate_map(&json!({ "metadata": "not-an-object" }))
            .unwrap_err(),
    );
    assert_eq!(fields.len(), 1);
    assert_eq!(fields[0].namespace().as_str(), "metadata");
    assert_eq!(fields[0].rule(), "type");
    assert_eq!(fields[0].params().text("expected"), Some("object"));
}

#[test]
fn schema_empty_array_fields_require_object_items_only_when_declared() {
    let object_items = Schema::from_yaml(
        r#"
fields:
  items:
    type: array
    fields: {}
"#,
    )
    .unwrap();
    let fields = fields(
        Validator::with_schema(object_items)
            .validate_map(&json!({ "items": ["scalar", null] }))
            .unwrap_err(),
    );
    assert_eq!(fields.len(), 2);
    assert_eq!(fields[0].namespace().as_str(), "items[0]");
    assert_eq!(fields[1].namespace().as_str(), "items[1]");
    assert!(fields.iter().all(|field| field.rule() == "type"));

    let unconstrained = Schema::from_yaml(
        r#"
fields:
  items:
    type: array
"#,
    )
    .unwrap();
    Validator::with_schema(unconstrained)
        .validate_map(&json!({ "items": ["scalar", null] }))
        .unwrap();
}

#[test]
fn schema_choice_shorthand_uses_values_param() -> Result<(), Box<dyn std::error::Error>> {
    let schema = Schema::from_yaml(
        r#"
fields:
  role:
    type: string
    rules:
      - noneof: [root, admin]
  state:
    type: string
    rules:
      - oneofci: [draft, published]
  username:
    type: string
    rules:
      - noneofci: [root, admin]
"#,
    )?;
    let fields = fields(
        Validator::with_schema(schema)
            .validate_map(&json!({
                "role": "root",
                "state": "archived",
                "username": "ADMIN"
            }))
            .unwrap_err(),
    );

    assert_eq!(fields.len(), 3);
    assert_eq!(param_list(&fields[0], "values"), vec!["root", "admin"]);
    assert_eq!(param_list(&fields[1], "values"), vec!["draft", "published"]);
    assert_eq!(param_list(&fields[2], "values"), vec!["root", "admin"]);
    assert!(
        fields
            .iter()
            .all(|field| field.params().text("value").is_none())
    );

    let messages = validator::i18n::en().render(&fields);
    assert!(
        messages
            .iter()
            .all(|message| !message.text.contains("{values}"))
    );

    Ok(())
}

#[test]
fn schema_unique_field_validates_array_object_fields() {
    let schema = Schema::from_yaml(
        r#"
fields:
  users:
    type: array
    rules:
      - unique: email
    fields:
      email:
        type: string
        rules:
          - required
          - email
"#,
    )
    .unwrap();
    let validator = Validator::with_schema(schema);

    validator
        .validate_map(&serde_json::json!({
            "users": [
                { "email": "first@example.com" },
                { "email": "second@example.com" }
            ]
        }))
        .unwrap();

    let fields = validator
        .validate_map(&serde_json::json!({
            "users": [
                { "email": "same@example.com" },
                { "email": "same@example.com" }
            ]
        }))
        .unwrap_err()
        .into_fields()
        .unwrap();
    assert_eq!(fields.len(), 1);
    assert_eq!(fields[0].namespace().as_str(), "users");
    assert_eq!(fields[0].rule(), "unique");
    assert_eq!(param_list(&fields[0], "fields"), vec!["email"]);
}

#[test]
fn schema_compound_unique_fields_compare_complete_nested_keys() {
    let schema = Schema::from_yaml(
        r#"
fields:
  users:
    type: array
    rules:
      - unique: [tenant_id, profile.email]
    fields:
      tenant_id:
        type: uint
      profile:
        type: object
        fields:
          email:
            type: string
"#,
    )
    .unwrap();
    let validator = Validator::with_schema(schema);

    validator
        .validate_map(&json!({
            "users": [
                { "tenant_id": 1, "profile": { "email": "same@example.com" } },
                { "tenant_id": 2, "profile": { "email": "same@example.com" } }
            ]
        }))
        .unwrap();

    let fields = fields(
        validator
            .validate_map(&json!({
                "users": [
                    { "tenant_id": 1, "profile": { "email": "same@example.com" } },
                    { "tenant_id": 1, "profile": { "email": "same@example.com" } }
                ]
            }))
            .unwrap_err(),
    );

    assert_eq!(fields.len(), 1);
    assert_eq!(fields[0].namespace().as_str(), "users");
    assert_eq!(fields[0].rule(), "unique");
    assert_eq!(
        param_list(&fields[0], "fields"),
        vec!["tenant_id", "profile.email"]
    );
    assert_eq!(fields[0].params().text("field"), None);
}

#[test]
fn schema_compound_unique_fields_preserve_none_and_malformed_data() {
    let schema = Schema::from_yaml(
        r#"
fields:
  users:
    type: array
    rules:
      - unique: [tenant_id, profile.email]
    fields:
      tenant_id:
        type: uint
      profile:
        type: object
        fields:
          email:
            type: string
"#,
    )
    .unwrap();
    let validator = Validator::with_schema(schema);

    validator
        .validate_map(&json!({
            "users": [
                { "tenant_id": 1 },
                { "tenant_id": 2, "profile": null }
            ]
        }))
        .unwrap();

    let none = fields(
        validator
            .validate_map(&json!({
                "users": [
                    { "tenant_id": 1 },
                    { "tenant_id": 1, "profile": null }
                ]
            }))
            .unwrap_err(),
    );
    assert_eq!(none.len(), 1);
    assert_eq!(none[0].rule(), "unique");

    let malformed = fields(
        validator
            .validate_map(&json!({
                "users": [
                    { "tenant_id": 1, "profile": "invalid" },
                    { "tenant_id": 1, "profile": "invalid" }
                ]
            }))
            .unwrap_err(),
    );
    assert_eq!(malformed.len(), 2);
    assert!(malformed.iter().all(|field| field.rule() == "type"));
    assert_eq!(malformed[0].namespace().as_str(), "users[0].profile");
    assert_eq!(malformed[1].namespace().as_str(), "users[1].profile");
}

#[test]
fn schema_alias_preserves_unique_field_context() {
    let schema = Schema::from_yaml(
        r#"
fields:
  users:
    type: array
    rules: unique_email
    fields:
      email:
        type: string
"#,
    )
    .unwrap();
    let validator = Validator::with_schema(schema)
        .alias("unique_email", "unique=email")
        .unwrap();

    let fields = validator
        .validate_map(&serde_json::json!({
            "users": [
                { "email": "same@example.com" },
                { "email": "same@example.com" }
            ]
        }))
        .unwrap_err()
        .into_fields()
        .unwrap();

    assert_eq!(fields.len(), 1);
    assert_eq!(fields[0].namespace().as_str(), "users");
    assert_eq!(fields[0].rule(), "unique_email");
    assert_eq!(fields[0].reason(), "unique");
    assert_eq!(param_list(&fields[0], "fields"), vec!["email"]);
}

#[test]
fn schema_alias_preserves_compound_unique_field_context() {
    let schema = Schema::from_yaml(
        r#"
fields:
  users:
    type: array
    rules: unique_account
    fields:
      tenant_id:
        type: uint
      profile:
        type: object
        fields:
          email:
            type: string
"#,
    )
    .unwrap();
    let validator = Validator::with_schema(schema)
        .alias("unique_account", "unique=tenant_id profile.email")
        .unwrap();

    let fields = fields(
        validator
            .validate_map(&json!({
                "users": [
                    { "tenant_id": 1, "profile": { "email": "same@example.com" } },
                    { "tenant_id": 1, "profile": { "email": "same@example.com" } }
                ]
            }))
            .unwrap_err(),
    );

    assert_eq!(fields.len(), 1);
    assert_eq!(fields[0].rule(), "unique_account");
    assert_eq!(fields[0].reason(), "unique");
    assert_eq!(
        param_list(&fields[0], "fields"),
        vec!["tenant_id", "profile.email"]
    );
}

#[test]
fn schema_array_fields_report_indexed_child_errors() {
    let schema = Schema::from_yaml(
        r#"
fields:
  users:
    type: array
    fields:
      email:
        type: string
        rules: email
"#,
    )
    .unwrap();
    let fields = Validator::with_schema(schema)
        .validate_map(&serde_json::json!({
            "users": [
                { "email": "invalid" },
                "not-an-object"
            ]
        }))
        .unwrap_err()
        .into_fields()
        .unwrap();

    assert_eq!(fields.len(), 2);
    assert_eq!(fields[0].namespace().as_str(), "users[0].email");
    assert_eq!(fields[0].rule(), "email");
    assert_eq!(fields[1].namespace().as_str(), "users[1]");
    assert_eq!(fields[1].rule(), "type");
    assert_eq!(fields[1].params().text("expected"), Some("object"));
}

#[test]
fn schema_array_fields_reject_null_items_as_indexed_type_errors() {
    let schema = Schema::from_yaml(
        r#"
fields:
  users:
    type: array
    fields:
      name:
        type: string
"#,
    )
    .unwrap();
    let fields = fields(
        Validator::with_schema(schema)
            .validate_map(&json!({ "users": [null] }))
            .unwrap_err(),
    );

    assert_eq!(fields.len(), 1);
    assert_eq!(fields[0].namespace().as_str(), "users[0]");
    assert_eq!(fields[0].kind(), Kind::Option);
    assert_eq!(fields[0].rule(), "type");
    assert_eq!(fields[0].params().text("expected"), Some("object"));
}

#[test]
fn schema_unique_field_leaves_invalid_values_to_child_type_validation() {
    let schema = Schema::from_yaml(
        r#"
fields:
  users:
    type: array
    rules:
      - unique: age
    fields:
      age:
        type: uint
"#,
    )
    .unwrap();
    let fields = fields(
        Validator::with_schema(schema)
            .validate_map(&json!({
                "users": [
                    { "age": "invalid" },
                    { "age": 1 }
                ]
            }))
            .unwrap_err(),
    );

    assert_eq!(fields.len(), 1);
    assert_eq!(fields[0].namespace().as_str(), "users[0].age");
    assert_eq!(fields[0].kind(), Kind::String);
    assert_eq!(fields[0].rule(), "type");
    assert_eq!(fields[0].params().text("expected"), Some("uint"));
}

#[test]
fn schema_unique_field_rejects_invalid_configuration() {
    for yaml in [
        r#"
fields:
  users:
    type: array
    rules:
      - unique: missing
    fields:
      email:
        type: string
"#,
        r#"
fields:
  users:
    type: object
    rules:
      - unique: email
    fields:
      email:
        type: string
"#,
        r#"
fields:
  users:
    type: array
    rules:
      - unique: profile
    fields:
      profile:
        type: object
        fields:
          email:
            type: string
"#,
        r#"
fields:
  users:
    type: array
    rules:
      - unique: profile.missing
    fields:
      profile:
        type: object
        fields:
          email:
            type: string
"#,
        r#"
fields:
  users:
    type: array
    rules:
      - unique: profile..email
    fields:
      profile:
        type: object
        fields:
          email:
            type: string
"#,
    ] {
        let schema = Schema::from_yaml(yaml).unwrap();
        let error = Validator::with_schema(schema)
            .validate_map(&serde_json::json!({ "users": [] }))
            .unwrap_err();
        assert!(matches!(error, Error::InvalidSchema { .. }));
    }
}

#[test]
fn schema_unique_fields_reject_invalid_lists() {
    let error = Schema::from_yaml(
        r#"
fields:
  users:
    type: array
    rules:
      - unique: []
    fields:
      email:
        type: string
"#,
    )
    .unwrap_err();
    assert!(matches!(error, Error::InvalidSchema { .. }));

    let schema = Schema::from_yaml(
        r#"
fields:
  users:
    type: array
    rules:
      - unique: [email, email]
    fields:
      email:
        type: string
"#,
    )
    .unwrap();
    let error = Validator::with_schema(schema)
        .validate_map(&json!({ "users": [] }))
        .unwrap_err();
    assert!(matches!(error, Error::InvalidRuleExpression { .. }));
}

#[test]
fn schema_unique_field_distinguishes_absent_and_empty_fields() {
    let without_fields = Schema::from_yaml(
        r#"
fields:
  users:
    type: array
    rules:
      - unique: email
"#,
    )
    .unwrap();
    let error = Validator::with_schema(without_fields)
        .validate_map(&json!({ "users": [] }))
        .unwrap_err();
    assert!(matches!(
        error,
        Error::InvalidSchema { reason }
            if reason.contains("requires an array with object fields")
    ));

    let empty_fields = Schema::from_yaml(
        r#"
fields:
  users:
    type: array
    rules:
      - unique: email
    fields: {}
"#,
    )
    .unwrap();
    let error = Validator::with_schema(empty_fields)
        .validate_map(&json!({ "users": [] }))
        .unwrap_err();
    assert!(matches!(
        error,
        Error::InvalidSchema { reason }
            if reason.contains("references undeclared field 'email'")
    ));
}

#[test]
fn schema_unique_field_treats_missing_and_null_as_none() {
    let schema = Schema::from_yaml(
        r#"
fields:
  users:
    type: array
    rules:
      - unique: nickname
    fields:
      nickname:
        type: string
"#,
    )
    .unwrap();
    let validator = Validator::with_schema(schema);

    validator
        .validate_map(&serde_json::json!({
            "users": [{}, { "nickname": "rust" }]
        }))
        .unwrap();

    let fields = validator
        .validate_map(&serde_json::json!({
            "users": [{}, { "nickname": null }]
        }))
        .unwrap_err()
        .into_fields()
        .unwrap();
    assert_eq!(fields.len(), 1);
    assert_eq!(fields[0].rule(), "unique");
    assert_eq!(param_list(&fields[0], "fields"), vec!["nickname"]);
}

#[test]
fn validate_map_without_schema_returns_error() {
    let error = Validator::new()
        .validate_map(&json!({ "title": "Rust" }))
        .unwrap_err();

    assert!(matches!(&error, validator::Error::MissingSchema));
    assert_eq!(
        error.to_string(),
        "schema is required for Schema validation"
    );
}

#[test]
fn schema_numeric_rules_follow_declared_families() {
    let unsigned = Schema::from_yaml(
        r#"
fields:
  count:
    type: uint
    rules:
      - min: -1
"#,
    )
    .unwrap();
    let error = Validator::with_schema(unsigned)
        .validate_map(&json!({}))
        .unwrap_err();
    assert!(matches!(error, Error::InvalidRuleExpression { .. }));

    let floating = Schema::from_yaml(
        r#"
fields:
  ratio:
    type: float
    rules:
      - min: 1.5
"#,
    )
    .unwrap();
    Validator::with_schema(floating)
        .validate_map(&json!({ "ratio": 2 }))
        .unwrap();
}

#[test]
fn schema_uint_cross_field_comparison_preserves_u64_range() {
    let schema = Schema::from_yaml(
        r#"
fields:
  lower:
    type: uint
  upper:
    type: uint
    rules:
      - gt_field: lower
"#,
    )
    .unwrap();
    let lower = i64::MAX as u64;
    let upper = lower + 1;

    Validator::with_schema(schema)
        .validate_map(&json!({ "lower": lower, "upper": upper }))
        .unwrap();
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
    assert_eq!(fields[0].kind(), Kind::String);
    assert_eq!(fields[0].rule(), "type");
    assert_eq!(fields[0].params().text("expected"), Some("object"));

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
    let error = Validator::new().validate_serde(&BrokenSerde).unwrap_err();

    assert!(matches!(error, validator::Error::MissingSchema));
}

#[test]
fn validate_serde_compiles_schema_before_serialization() {
    let schema = Schema::from_yaml(
        r#"
fields:
  title:
    type: string
    rules: missing_rule
"#,
    )
    .unwrap();
    let error = Validator::with_schema(schema)
        .validate_serde(&BrokenSerde)
        .unwrap_err();

    assert!(matches!(error, Error::UnknownRule { name } if name == "missing_rule"));
}

#[test]
fn direct_value_uses_new_common_rules() {
    Validator::new()
        .value(&"https://example.com", "required,http,https")
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
    assert_eq!(fields[0].params().text("value"), Some("!@#?"));
    assert_eq!(fields[1].rule(), "noneof");
    assert_eq!(param_list(&fields[1], "values"), vec!["root", "admin"]);
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
      - https
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
            ("source_url", "https"),
            ("state", "eq"),
            ("tags", "unique"),
            ("username", "noneof"),
        ]
    );
    assert_eq!(fields[2].params().text("value"), Some("-"));
    assert_eq!(param_list(&fields[10], "values"), vec!["1", "2", "3"]);
    assert_eq!(fields[16].params().text("value"), Some("published"));
    assert_eq!(param_list(&fields[18], "values"), vec!["root", "admin"]);

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
    type: int
  end_at:
    type: int
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
    assert_eq!(fields[0].params().text("compare"), Some("password"));
    assert_eq!(fields[1].namespace().as_str(), "end_at");
    assert_eq!(fields[1].rule(), "gt_field");
    assert_eq!(fields[1].params().text("compare"), Some("start_at"));

    Ok(())
}

#[test]
fn schema_cross_field_rule_passes() -> Result<(), Box<dyn std::error::Error>> {
    let schema = Schema::from_yaml(
        r#"
fields:
  start_at:
    type: int
  end_at:
    type: int
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
fn schema_field_string_rules_compare_sibling_fields() -> Result<(), Box<dyn std::error::Error>> {
    let schema = Schema::from_yaml(
        r#"
fields:
  needle:
    type: string
  forbidden:
    type: string
  body:
    type: string
    rules:
      - fieldcontains: needle
      - fieldexcludes: forbidden
"#,
    )?;
    let fields = fields(
        Validator::with_schema(schema)
            .validate_map(&json!({
                "needle": "rust",
                "forbidden": "go",
                "body": "hello go"
            }))
            .unwrap_err(),
    );
    let rules = fields.iter().map(|field| field.rule()).collect::<Vec<_>>();

    assert_eq!(rules, vec!["fieldcontains", "fieldexcludes"]);
    assert_eq!(fields[0].params().text("compare"), Some("needle"));
    assert_eq!(fields[1].params().text("compare"), Some("forbidden"));

    Ok(())
}

#[test]
fn schema_conditional_field_rules_validate_sibling_fields() -> Result<(), Box<dyn std::error::Error>>
{
    let schema = Schema::from_yaml(
        r#"
fields:
  status:
    type: string
  email:
    type: string
  phone:
    type: string
  published_at:
    type: string
    rules:
      - required_if:
          status: published
  title:
    type: string
    rules:
      - required_unless:
          status: draft
  reviewer:
    type: string
    rules:
      - skip_unless:
          status: published
  contact_name:
    type: string
    rules:
      - required_with: [email, phone]
  fallback_contact:
    type: string
    rules:
      - required_without: [email, phone]
"#,
    )?;

    Validator::with_schema(schema.clone()).validate_map(&json!({
        "status": "draft",
        "email": "",
        "fallback_contact": "support"
    }))?;

    let fields = fields(
        Validator::with_schema(schema)
            .validate_map(&json!({
                "status": "published",
                "email": "editor@example.com"
            }))
            .unwrap_err(),
    );
    let mut rules = fields.iter().map(|field| field.rule()).collect::<Vec<_>>();
    rules.sort_unstable();

    assert_eq!(
        rules,
        vec![
            "required_if",
            "required_unless",
            "required_with",
            "required_without",
            "skip_unless"
        ]
    );
    assert_eq!(
        fields
            .iter()
            .find(|field| field.rule() == "required_if")
            .and_then(|field| param_pair(field, "status")),
        Some("published")
    );
    assert_eq!(
        fields
            .iter()
            .find(|field| field.rule() == "required_unless")
            .and_then(|field| param_pair(field, "status")),
        Some("draft")
    );
    for rule in ["required_with", "required_without"] {
        let field = fields
            .iter()
            .find(|field| field.rule() == rule)
            .expect("expected conditional field error");
        assert_eq!(param_list(field, "fields"), vec!["email", "phone"]);
    }
    assert_eq!(
        fields
            .iter()
            .find(|field| field.rule() == "skip_unless")
            .and_then(|field| param_pair(field, "status")),
        Some("published")
    );

    Ok(())
}

#[test]
fn schema_conditional_all_and_excluded_rules_validate_sibling_fields()
-> Result<(), Box<dyn std::error::Error>> {
    let schema = Schema::from_yaml(
        r#"
fields:
  status:
    type: string
  mode:
    type: string
  email:
    type: string
  phone:
    type: string
  backup_email:
    type: string
  backup_phone:
    type: string
  contact_name:
    type: string
    rules:
      - required_with_all: [email, phone]
  fallback_contact:
    type: string
    rules:
      - required_without_all: [backup_email, backup_phone]
  archive_note:
    type: string
    rules:
      - excluded_if:
          status: archived
  draft_only_note:
    type: string
    rules:
      - excluded_unless:
          status: draft
  contact_override:
    type: string
    rules:
      - excluded_with: [email, phone]
  all_contact_override:
    type: string
    rules:
      - excluded_with_all: [email, phone]
  backup_missing_override:
    type: string
    rules:
      - excluded_without: [backup_email, backup_phone]
  backup_all_missing_override:
    type: string
    rules:
      - excluded_without_all: [backup_email, backup_phone]
  private_note:
    type: string
    rules:
      - excluded_if:
          mode: private
"#,
    )?;

    Validator::with_schema(schema.clone()).validate_map(&json!({
        "status": "draft",
        "mode": "public",
        "email": "editor@example.com",
        "backup_email": "backup@example.com",
        "contact_name": "",
        "fallback_contact": "",
        "archive_note": "",
        "draft_only_note": "draft note",
        "contact_override": "",
        "all_contact_override": "",
        "backup_missing_override": "",
        "backup_all_missing_override": "",
        "private_note": ""
    }))?;

    let fields = fields(
        Validator::with_schema(schema)
            .validate_map(&json!({
                "status": "archived",
                "mode": "private",
                "email": "editor@example.com",
                "phone": "123",
                "backup_email": "",
                "contact_name": "",
                "fallback_contact": "",
                "archive_note": "archived",
                "draft_only_note": "draft only",
                "contact_override": "override",
                "all_contact_override": "all override",
                "backup_missing_override": "missing",
                "backup_all_missing_override": "all missing",
                "private_note": "secret"
            }))
            .unwrap_err(),
    );
    let mut failures = fields
        .iter()
        .map(|field| {
            (
                field.rule(),
                field
                    .params()
                    .list("fields")
                    .map(|values| values.iter().map(String::as_str).collect::<Vec<_>>()),
            )
        })
        .collect::<Vec<_>>();
    failures.sort_unstable();

    assert_eq!(
        failures,
        vec![
            ("excluded_if", None),
            ("excluded_if", None),
            ("excluded_unless", None),
            ("excluded_with", Some(vec!["email", "phone"])),
            ("excluded_with_all", Some(vec!["email", "phone"])),
            (
                "excluded_without",
                Some(vec!["backup_email", "backup_phone"])
            ),
            (
                "excluded_without_all",
                Some(vec!["backup_email", "backup_phone"])
            ),
            ("required_with_all", Some(vec!["email", "phone"])),
            (
                "required_without_all",
                Some(vec!["backup_email", "backup_phone"])
            ),
        ]
    );

    Ok(())
}

#[test]
fn schema_conditional_field_rule_undeclared_target_returns_config_error()
-> Result<(), Box<dyn std::error::Error>> {
    let schema = Schema::from_yaml(
        r#"
fields:
  title:
    type: string
    rules:
      - required_with: [missing]
"#,
    )?;
    let error = Validator::with_schema(schema)
        .validate_map(&json!({
            "title": ""
        }))
        .unwrap_err();

    assert!(matches!(
        error,
        validator::Error::InvalidSchema { reason }
            if reason.contains("references undeclared field 'missing'")
    ));

    Ok(())
}

#[test]
fn schema_fieldexcludes_missing_target_value_passes() -> Result<(), Box<dyn std::error::Error>> {
    let schema = Schema::from_yaml(
        r#"
fields:
  forbidden:
    type: string
  body:
    type: string
    rules:
      - fieldexcludes: forbidden
"#,
    )?;

    Validator::with_schema(schema).validate_map(&json!({
        "body": "hello go"
    }))?;

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
    assert_eq!(fields[0].params().text("compare"), Some("password"));

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
