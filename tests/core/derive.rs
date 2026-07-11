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
#[validate(check = "check_unknown_field")]
struct CheckedFieldName {
    known: String,
}

fn check_unknown_field(
    value: &CheckedFieldName, valid: &mut validator::valid::Valid<'_>,
) {
    let _ = &value.known;
    valid.field("unknown").rule("custom").push();
}

#[test]
fn struct_level_check_rejects_unknown_field() {
    let error = Validator::new()
        .validate(&CheckedFieldName {
            known: String::new(),
        })
        .unwrap_err();

    assert!(matches!(
        error,
        Error::UnknownField { field } if field == "unknown"
    ));
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

