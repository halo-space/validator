use halo::validator::prelude::*;
use serde_json::json;

#[derive(Debug)]
struct Slug;

struct SlugProvider;

impl Provider for SlugProvider {
    fn register(&self, config: Config) -> Result<Config, Error> {
        config.rule("slug", Slug)
    }
}

struct AsyncPass;

impl Rule for AsyncPass {
    async fn check(&self, _field: &Field<'_>) -> Result<bool, Error> {
        Ok(true)
    }
}

impl Rule for Slug {
    async fn check(&self, field: &Field<'_>) -> Result<bool, Error> {
        Ok(field.value().string().is_some_and(|value| {
            !value.is_empty()
                && value
                    .chars()
                    .all(|ch| ch.is_ascii_lowercase() || ch.is_ascii_digit() || ch == '-')
        }))
    }
}

#[derive(Debug, Validate)]
struct Post {
    #[validate(slug)]
    slug: String,
}

#[derive(Debug)]
struct StartsWith;

impl Rule for StartsWith {
    fn signature(&self) -> Signature {
        Signature::text("prefix")
    }

    async fn check(&self, field: &Field<'_>) -> Result<bool, Error> {
        let Some(prefix) = field.params().text("prefix") else {
            return Ok(false);
        };
        Ok(field
            .value()
            .string()
            .is_some_and(|value| value.starts_with(prefix)))
    }
}

#[derive(Debug, Validate)]
struct Article {
    #[validate(starts_with = "post-")]
    slug: String,
}

#[derive(Debug, Validate)]
struct InvalidLength {
    #[validate(length(mian = 3))]
    value: String,
}

#[derive(Debug)]
struct SameField;

impl Rule for SameField {
    fn signature(&self) -> Signature {
        Signature::field("compare")
    }

    async fn check(&self, field: &Field<'_>) -> Result<bool, Error> {
        let target = field.params().text("compare").unwrap();
        Ok(field.sibling(target).and_then(Value::string) == field.value().string())
    }
}

#[derive(Debug, Validate)]
struct MatchingFields {
    left: String,
    #[validate(same_field = "left")]
    right: String,
}

#[derive(Debug, Validate)]
struct ExplicitMatchingFields {
    left: String,
    #[validate(field(name = "same", target = "left"))]
    right: String,
}

#[derive(Debug, Validate)]
struct AliasMatchingFields {
    #[allow(dead_code)]
    left: String,
    #[validate(alias = "same")]
    right: String,
}

#[derive(Debug, Validate)]
struct InvalidNumericParameter {
    #[validate(min = "invalid")]
    value: i32,
}

#[derive(Debug, Validate)]
struct InvalidLengthBounds {
    #[validate(omitempty, length(exact = 3, min = 1))]
    value: String,
}

#[derive(Debug, Validate)]
struct InvalidSkippedRegex {
    #[validate(omitempty, regex(pattern = "["))]
    value: String,
}

#[derive(Debug, Validate)]
struct InvalidConditionParameter {
    age: u8,

    #[validate(required_if(age = "invalid"))]
    name: String,
}

#[derive(Debug, Validate)]
struct InvalidEmptyDive {
    #[validate(dive(regex(pattern = "[")))]
    values: Vec<String>,
}

#[derive(Debug, Validate)]
struct InvalidEmptyMapDive {
    #[validate(dive(keys(required), values(min = "invalid")))]
    values: std::collections::HashMap<String, u32>,
}

#[derive(Debug)]
struct RejectParams;

impl Rule for RejectParams {
    fn validate_params(&self, _field: &Field<'_>) -> Result<(), Error> {
        Err(Error::InvalidRuleExpression {
            expression: "reject_params".to_owned(),
            reason: "custom parameter rejection".to_owned(),
        })
    }

    async fn check(&self, _field: &Field<'_>) -> Result<bool, Error> {
        panic!("check must not run after parameter preflight fails")
    }
}

#[derive(Debug, Validate)]
struct InvalidCustomParams {
    #[validate(reject_params)]
    value: String,
}

#[tokio::test]
async fn custom_marker_rule_uses_the_same_contract_in_every_entry() {
    let derive = Validator::new()
        .rule("slug", Slug)
        .unwrap()
        .validate(&Post {
            slug: "Hello World".to_owned(),
        })
        .await
        .unwrap_err()
        .into_fields()
        .unwrap();

    let direct = Validator::new()
        .rule("slug", Slug)
        .unwrap()
        .value(&"Hello World", "slug")
        .await
        .unwrap_err()
        .into_fields()
        .unwrap();

    let schema = Schema::from_yaml(
        r#"
fields:
  slug:
    type: string
    rules: slug
"#,
    )
    .unwrap();
    let schema = Validator::with_schema(schema)
        .rule("slug", Slug)
        .unwrap()
        .check_map(&json!({ "slug": "Hello World" }))
        .await
        .unwrap_err()
        .into_fields()
        .unwrap();

    for fields in [&derive, &direct, &schema] {
        assert_eq!(fields.len(), 1);
        assert_eq!(fields[0].rule(), "slug");
        assert_eq!(fields[0].reason(), "slug");
        assert!(fields[0].params().is_empty());
    }
}

#[tokio::test]
async fn custom_parameter_rule_uses_the_same_contract_in_every_entry() {
    let derive = Validator::new()
        .rule("starts_with", StartsWith)
        .unwrap()
        .validate(&Article {
            slug: "entry-1".to_owned(),
        })
        .await
        .unwrap_err()
        .into_fields()
        .unwrap();

    let direct = Validator::new()
        .rule("starts_with", StartsWith)
        .unwrap()
        .value(&"entry-1", "starts_with=post-")
        .await
        .unwrap_err()
        .into_fields()
        .unwrap();

    let schema = Schema::from_yaml(
        r#"
fields:
  slug:
    type: string
    rules:
      - starts_with: post-
"#,
    )
    .unwrap();
    let schema = Validator::with_schema(schema)
        .rule("starts_with", StartsWith)
        .unwrap()
        .check_map(&json!({ "slug": "entry-1" }))
        .await
        .unwrap_err()
        .into_fields()
        .unwrap();

    for fields in [&derive, &direct, &schema] {
        assert_eq!(fields.len(), 1);
        assert_eq!(fields[0].rule(), "starts_with");
        assert_eq!(fields[0].reason(), "starts_with");
        assert_eq!(fields[0].params().text("prefix"), Some("post-"));
    }
}

#[tokio::test]
async fn custom_field_rule_uses_explicit_derive_field_access() {
    Validator::new()
        .rule("same_field", SameField)
        .unwrap()
        .validate(&MatchingFields {
            left: "same".to_owned(),
            right: "same".to_owned(),
        })
        .await
        .unwrap();

    let schema = Schema::from_yaml(
        r#"
fields:
  left:
    type: string
  right:
    type: string
    rules:
      - same_field: left
"#,
    )
    .unwrap();
    Validator::with_schema(schema)
        .rule("same_field", SameField)
        .unwrap()
        .check_map(&json!({ "left": "same", "right": "same" }))
        .await
        .unwrap();

    let error = Validator::new()
        .rule("same_field", SameField)
        .unwrap()
        .value(&"same", "same_field=left")
        .await
        .unwrap_err();
    assert!(matches!(error, Error::MissingFieldContext { .. }));
}

#[tokio::test]
async fn derive_alias_cannot_hide_field_access() {
    let error = Validator::new()
        .alias("same", "eq_field=left")
        .unwrap()
        .validate(&AliasMatchingFields {
            left: "same".to_owned(),
            right: "same".to_owned(),
        })
        .await
        .unwrap_err();

    assert!(matches!(error, Error::MissingFieldContext { name } if name == "eq_field"));
}

#[tokio::test]
async fn signatures_reject_unknown_missing_and_extra_parameters() {
    for expression in ["min(mian=3)", "min", "email(foo=bar)"] {
        let error = Validator::new()
            .value(&"rust", expression)
            .await
            .unwrap_err();
        assert!(matches!(error, Error::InvalidRuleExpression { .. }));
    }

    let derive = Validator::new()
        .validate(&InvalidLength {
            value: "rust".to_owned(),
        })
        .await
        .unwrap_err();
    assert!(matches!(derive, Error::InvalidRuleExpression { .. }));

    let schema = Schema::from_yaml(
        r#"
fields:
  value:
    type: string
    rules:
      - min:
          mian: 3
"#,
    )
    .unwrap();
    let schema = Validator::with_schema(schema)
        .check_map(&json!({ "value": "rust" }))
        .await
        .unwrap_err();
    assert!(matches!(schema, Error::InvalidRuleExpression { .. }));
}

#[tokio::test]
async fn semantic_parameter_errors_remain_configuration_errors() {
    let direct = Validator::new()
        .value(&5_i32, "min=invalid")
        .await
        .unwrap_err();
    assert!(matches!(direct, Error::InvalidRuleExpression { .. }));

    let derive = Validator::new()
        .validate(&InvalidNumericParameter { value: 5 })
        .await
        .unwrap_err();
    assert!(matches!(derive, Error::InvalidRuleExpression { .. }));

    let schema = Schema::from_yaml(
        r#"
fields:
  value:
    type: int
    rules:
      - min: invalid
"#,
    )
    .unwrap();
    let schema = Validator::with_schema(schema)
        .check_map(&json!({ "value": 5 }))
        .await
        .unwrap_err();
    assert!(matches!(schema, Error::InvalidRuleExpression { .. }));

    let validator = Validator::new();
    for _ in 0..2 {
        let regex = validator
            .value(&"value", r#"regex(pattern="[")"#)
            .await
            .unwrap_err();
        assert!(matches!(regex, Error::InvalidRuleExpression { .. }));
    }
}

#[tokio::test]
async fn parameter_preflight_runs_before_every_data_short_circuit() {
    let validator = Validator::new();

    for error in [
        validator
            .value(&Option::<i32>::None, "min=invalid")
            .await
            .unwrap_err(),
        validator
            .value(&String::new(), r#"omitempty,regex(pattern="[")"#)
            .await
            .unwrap_err(),
        validator
            .value(&"ok", r#"eq=ok|regex("[")"#)
            .await
            .unwrap_err(),
        validator
            .value(&5_i32, "range(min=10,max=invalid)")
            .await
            .unwrap_err(),
        validator
            .value(&1_i32, "oneof(1,invalid)")
            .await
            .unwrap_err(),
        validator
            .value(&serde_json::Value::Null, "omitempty,range(min=10,max=1)")
            .await
            .unwrap_err(),
    ] {
        assert!(matches!(error, Error::InvalidRuleExpression { .. }));
    }

    let derive = validator
        .validate(&InvalidSkippedRegex {
            value: String::new(),
        })
        .await
        .unwrap_err();
    assert!(matches!(derive, Error::InvalidRuleExpression { .. }));

    let condition = validator
        .validate(&InvalidConditionParameter {
            age: 1,
            name: String::new(),
        })
        .await
        .unwrap_err();
    assert!(matches!(condition, Error::InvalidRuleExpression { .. }));

    let bounds = validator
        .validate(&InvalidLengthBounds {
            value: String::new(),
        })
        .await
        .unwrap_err();
    assert!(matches!(bounds, Error::InvalidRuleExpression { .. }));

    let schema = Schema::from_yaml(
        r#"
fields:
  value:
    type: int
    rules:
      - omitempty
      - range:
          min: 10
          max: 1
"#,
    )
    .unwrap();
    let schema = Validator::with_schema(schema)
        .check_map(&json!({}))
        .await
        .unwrap_err();
    assert!(matches!(schema, Error::InvalidRuleExpression { .. }));
}

#[tokio::test]
async fn unique_kind_preflight_runs_in_schema_before_omitempty() {
    let schema = Schema::from_yaml(
        r#"
fields:
  value:
    type: int
    rules: omitempty,unique
"#,
    )
    .unwrap();
    let error = Validator::with_schema(schema)
        .check_map(&json!({}))
        .await
        .unwrap_err();

    assert!(matches!(error, Error::InvalidRuleExpression { .. }));
}

#[tokio::test]
async fn empty_dive_still_preflights_element_rules() {
    let validator = Validator::new();
    let error = validator
        .validate(&InvalidEmptyDive { values: Vec::new() })
        .await
        .unwrap_err();
    assert!(matches!(error, Error::InvalidRuleExpression { .. }));

    let error = validator
        .validate(&InvalidEmptyMapDive {
            values: std::collections::HashMap::new(),
        })
        .await
        .unwrap_err();
    assert!(matches!(error, Error::InvalidRuleExpression { .. }));
}

#[tokio::test]
async fn custom_parameter_preflight_runs_in_every_entry() {
    let derive = Validator::new()
        .rule("reject_params", RejectParams)
        .unwrap()
        .validate(&InvalidCustomParams {
            value: "value".to_owned(),
        })
        .await
        .unwrap_err();
    assert!(
        matches!(derive, Error::InvalidRuleExpression { reason, .. } if reason == "custom parameter rejection")
    );

    let direct = Validator::new()
        .rule("reject_params", RejectParams)
        .unwrap()
        .value(&"value", "reject_params")
        .await
        .unwrap_err();
    assert!(
        matches!(direct, Error::InvalidRuleExpression { reason, .. } if reason == "custom parameter rejection")
    );

    let schema = Schema::from_yaml(
        r#"
fields:
  value:
    type: string
    rules: reject_params
"#,
    )
    .unwrap();
    let schema = Validator::with_schema(schema)
        .rule("reject_params", RejectParams)
        .unwrap()
        .check_map(&json!({ "value": "value" }))
        .await
        .unwrap_err();
    assert!(
        matches!(schema, Error::InvalidRuleExpression { reason, .. } if reason == "custom parameter rejection")
    );
}

#[tokio::test]
async fn schema_parameter_preflight_precedes_root_and_field_type_checks() {
    let schema = Schema::from_yaml(
        r#"
fields:
  value:
    type: string
    rules:
      - regex: "["
"#,
    )
    .unwrap();
    let validator = Validator::with_schema(schema);

    for data in [json!("not-an-object"), json!({ "value": 42 })] {
        let error = validator.check_map(&data).await.unwrap_err();
        assert!(matches!(error, Error::InvalidRuleExpression { .. }));
    }
}

#[tokio::test]
async fn choice_values_keep_delimiter_characters() {
    Validator::new()
        .value(&"a,b", r#"oneof("a,b","c")"#)
        .await
        .unwrap();
    Validator::new()
        .value(&"a=b", r#"oneof("a=b","c")"#)
        .await
        .unwrap();

    let fields = Validator::new()
        .value(&"other", r#"oneof("a,b","c")"#)
        .await
        .unwrap_err()
        .into_fields()
        .unwrap();
    let values = fields[0].params().list("values").unwrap();

    assert_eq!(values, &["a,b", "c"]);

    Validator::new()
        .value(&"a\"b", r#"oneof("a\"b","c\\d")"#)
        .await
        .unwrap();
}

#[tokio::test]
async fn empty_alias_and_malformed_expressions_are_rejected() {
    for expression in ["", "email,", "email||url", r#"oneof("value)"#] {
        let error = Validator::new()
            .value(&"value", expression)
            .await
            .unwrap_err();
        assert!(matches!(error, Error::InvalidRuleExpression { .. }));
    }

    let Err(error) = Validator::new().alias("empty", "") else {
        panic!("empty alias must be rejected");
    };
    assert!(matches!(error, Error::InvalidRuleExpression { .. }));
}

#[tokio::test]
async fn field_rule_requires_field_context() {
    let error = Validator::new()
        .value(&"value", "eq_field=other")
        .await
        .unwrap_err();

    assert!(matches!(
        error,
        Error::MissingFieldContext { name } if name == "eq_field"
    ));
}

#[tokio::test]
async fn recursive_aliases_are_rejected() {
    let direct = Validator::new()
        .alias("cycle", "cycle")
        .unwrap()
        .value(&"value", "cycle")
        .await
        .unwrap_err();
    assert!(matches!(
        direct,
        Error::RecursiveAlias { name } if name == "cycle"
    ));

    let indirect = Validator::new()
        .alias("first", "second")
        .unwrap()
        .alias("second", "first")
        .unwrap()
        .value(&"value", "first")
        .await
        .unwrap_err();
    assert!(matches!(
        indirect,
        Error::RecursiveAlias { name } if name == "first"
    ));
}

#[tokio::test]
async fn nested_alias_preserves_outer_rule() {
    let fields = Validator::new()
        .alias("inner", "email")
        .unwrap()
        .alias("outer", "inner")
        .unwrap()
        .value(&"invalid", "outer")
        .await
        .unwrap_err()
        .into_fields()
        .unwrap();

    assert_eq!(fields[0].rule(), "outer");
    assert_eq!(fields[0].reason(), "email");
}

#[tokio::test]
async fn built_in_and_control_names_cannot_be_overridden() {
    for name in [
        "required",
        "alias",
        "check",
        "dive",
        "keys",
        "nested",
        "omitempty",
        "values",
    ] {
        let Err(error) = Validator::new().rule(name, Slug) else {
            panic!("expected duplicate name error");
        };
        assert!(matches!(error, Error::DuplicateName { name: actual } if actual == name));

        let Err(error) = Validator::new().alias(name, "email") else {
            panic!("expected duplicate name error");
        };
        assert!(matches!(error, Error::DuplicateName { name: actual } if actual == name));
    }
}

#[tokio::test]
async fn provider_extends_config_before_validator_creation() {
    let validator = Validator::from(Config::default().provider(SlugProvider).unwrap());
    validator
        .validate(&Post {
            slug: "valid-slug".to_owned(),
        })
        .await
        .unwrap();
}

#[tokio::test]
async fn explicit_field_rule_syntax_supports_custom_rule_names() {
    let value = ExplicitMatchingFields {
        left: "same".to_owned(),
        right: "same".to_owned(),
    };

    Validator::new()
        .rule("same", SameField)
        .unwrap()
        .validate(&value)
        .await
        .unwrap();
}

#[tokio::test]
async fn rule_runs_through_value_entry_point() {
    let validator = Validator::new().rule("async_pass", AsyncPass).unwrap();
    validator.value(&"value", "async_pass").await.unwrap();
}

#[tokio::test]
async fn cache_stats_report_hits_and_misses() {
    let validator = Validator::new();
    validator
        .value(&"valid@example.com", "email")
        .await
        .unwrap();
    validator
        .value(&"valid@example.com", "email")
        .await
        .unwrap();

    let stats = validator.stats();
    assert!(stats.expression_misses >= 1);
    assert!(stats.compiled_misses >= 1);
    assert!(stats.compiled_hits >= 1);
}
