
#[derive(Debug, Validate)]
struct PublishState {
    #[validate(oneof("draft", "published"))]
    status: String,
}

#[derive(Debug, Validate)]
struct Priority {
    #[validate(oneof(1, 2, 3))]
    level: u8,
}

#[test]
fn oneof_passes_for_allowed_value() {
    let value = PublishState {
        status: "draft".to_owned(),
    };

    Validator::new().validate(&value).unwrap();
}

#[test]
fn oneof_reports_values() {
    let value = PublishState {
        status: "archived".to_owned(),
    };

    let fields = Validator::new()
        .validate(&value)
        .unwrap_err()
        .into_fields()
        .unwrap();

    assert_eq!(fields.len(), 1);
    assert_eq!(fields[0].rule(), "oneof");
    assert_eq!(param_list(&fields[0], "values"), vec!["draft", "published"]);
}

#[test]
fn oneof_dispatches_integer_candidates_by_field_type() {
    let value = Priority { level: 2 };

    Validator::new().validate(&value).unwrap();
}

#[derive(Debug, Validate)]
struct ReservedUsername {
    #[validate(noneof("root", "admin"))]
    username: String,
}

#[derive(Debug, Validate)]
struct ForbiddenLevel {
    #[validate(noneof(1, 2, 3))]
    level: i32,
}

#[test]
fn noneof_passes_for_unlisted_value() {
    let value = ReservedUsername {
        username: "alice".to_owned(),
    };

    Validator::new().validate(&value).unwrap();
}

#[test]
fn noneof_reports_values() {
    let value = ReservedUsername {
        username: "root".to_owned(),
    };

    let fields = Validator::new()
        .validate(&value)
        .unwrap_err()
        .into_fields()
        .unwrap();

    assert_eq!(fields.len(), 1);
    assert_eq!(fields[0].rule(), "noneof");
    assert_eq!(param_list(&fields[0], "values"), vec!["root", "admin"]);
}

#[test]
fn noneof_dispatches_integer_candidates_by_field_type() {
    let value = ForbiddenLevel { level: 4 };

    Validator::new().validate(&value).unwrap();

    let value = ForbiddenLevel { level: 2 };
    let fields = Validator::new()
        .validate(&value)
        .unwrap_err()
        .into_fields()
        .unwrap();

    assert_eq!(fields.len(), 1);
    assert_eq!(fields[0].rule(), "noneof");
    assert_eq!(param_list(&fields[0], "values"), vec!["1", "2", "3"]);
}

#[test]
fn case_insensitive_choice_rules_work_for_strings() {
    let validator = Validator::new();

    validator.value(&"ReD", "oneofci(red,green)").unwrap();
    validator
        .value(&"BLUE", "oneofci('red green',blue)")
        .unwrap();
    validator.value(&"yellow", "noneofci(red,green)").unwrap();

    let fields = validator
        .value(&"RED", "noneofci(red,green)")
        .unwrap_err()
        .into_fields()
        .unwrap();

    assert_eq!(fields[0].rule(), "noneofci");
    assert_eq!(param_list(&fields[0], "values"), vec!["red", "green"]);
}

#[derive(Debug, Validate)]
struct ChoiceCaseRules {
    #[validate(oneofci("draft", "published"))]
    state: String,

    #[validate(noneofci("root", "admin"))]
    username: String,
}

#[test]
fn derive_case_insensitive_choice_rules_work() {
    let value = ChoiceCaseRules {
        state: "PUBLISHED".to_owned(),
        username: "ADMIN".to_owned(),
    };

    let fields = Validator::new()
        .validate(&value)
        .unwrap_err()
        .into_fields()
        .unwrap();

    assert_eq!(fields.len(), 1);
    assert_eq!(fields[0].rule(), "noneofci");
}

#[derive(Debug, Validate)]
struct AliasState {
    #[validate(alias = "publish_state")]
    status: String,
}

#[test]
fn oneof_works_inside_alias_expression() -> Result<(), Box<dyn std::error::Error>> {
    let value = AliasState {
        status: "archived".to_owned(),
    };

    let fields = Validator::new()
        .alias("publish_state", r#"oneof("draft","published")"#)?
        .validate(&value)
        .unwrap_err()
        .into_fields()
        .unwrap();

    assert_eq!(fields.len(), 1);
    assert_eq!(fields[0].rule(), "publish_state");
    assert_eq!(fields[0].reason(), "oneof");
    assert_eq!(param_list(&fields[0], "values"), vec!["draft", "published"]);

    Ok(())
}
