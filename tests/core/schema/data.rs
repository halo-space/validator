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
    let validator = Validator::with_schema(floating);
    validator.validate_map(&json!({ "ratio": 2.0 })).unwrap();

    let fields = validator
        .validate_map(&json!({ "ratio": 2 }))
        .unwrap_err()
        .into_fields()
        .unwrap();
    assert_eq!(fields[0].rule(), "type");
    assert_eq!(fields[0].params().text("expected"), Some("float"));

    let signed = Schema::from_yaml(
        r#"
fields:
  count:
    type: int
"#,
    )
    .unwrap();
    let fields = Validator::with_schema(signed)
        .validate_map(&json!({ "count": 2.0 }))
        .unwrap_err()
        .into_fields()
        .unwrap();
    assert_eq!(fields[0].rule(), "type");
    assert_eq!(fields[0].params().text("expected"), Some("int"));
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

