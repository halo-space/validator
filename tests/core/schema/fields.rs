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

