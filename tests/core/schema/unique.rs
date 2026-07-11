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

