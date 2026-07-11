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
fn schema_rejects_unsupported_type_names() {
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

