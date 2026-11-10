#[tokio::test]
async fn direct_value_passes() -> Result<(), Box<dyn std::error::Error>> {
    Validator::new().value(&"team@example.com", "required,email").await?;

    Ok(())
}

#[tokio::test]
async fn direct_value_reports_value_namespace() {
    let fields = fields(
        Validator::new()
            .value(&"not-email", "required,email").await
            .unwrap_err(),
    );

    assert_eq!(fields.len(), 1);
    assert_eq!(fields[0].namespace().as_str(), "$value");
    assert_eq!(fields[0].field(), "$value");
    assert_eq!(fields[0].rule(), "email");
}

#[tokio::test]
async fn repeated_direct_value_validation_preserves_errors() {
    let validator = Validator::new();
    validator
        .value(&"team@example.com", "required,email").await
        .unwrap();

    let first = fields(validator.value(&"not-email", "required,email").await.unwrap_err());
    let second = fields(validator.value(&"not-email", "required,email").await.unwrap_err());

    assert_eq!(first, second);
}

#[tokio::test]
async fn cached_groups_remain_valid_after_registry_extension() -> Result<(), Box<dyn std::error::Error>> {
    let validator = Validator::new();
    validator.value(&"team@example.com", "required,email").await?;

    let validator = validator
        .rule("slug", Slug)?
        .alias("mail", "required,email")?;

    validator.value(&"team@example.com", "required,email").await?;
    validator.value(&"hello-rust", "slug").await?;
    validator.value(&"team@example.com", "mail").await?;

    Ok(())
}

#[tokio::test]
async fn direct_value_omitempty_skips_empty_value() -> Result<(), Box<dyn std::error::Error>> {
    Validator::new().value(&String::new(), "omitempty,email").await?;

    Ok(())
}

#[tokio::test]
async fn direct_value_alias_preserves_rule_and_reason() -> Result<(), Box<dyn std::error::Error>> {
    let fields = fields(
        Validator::new()
            .alias("username", "required,length(min=3,max=20)")?
            .value(&"al", "username").await
            .unwrap_err(),
    );

    assert_eq!(fields.len(), 1);
    assert_eq!(fields[0].rule(), "username");
    assert_eq!(fields[0].reason(), "length");
    assert_eq!(fields[0].params().text("min"), Some("3"));

    Ok(())
}

#[tokio::test]
async fn direct_value_alias_omitempty_skips_empty_value() -> Result<(), Box<dyn std::error::Error>> {
    Validator::new()
        .alias("optional_email", "omitempty,email")?
        .value(&String::new(), "optional_email").await?;

    Ok(())
}

#[tokio::test]
async fn direct_value_alias_omitempty_skips_following_rules() -> Result<(), Box<dyn std::error::Error>> {
    Validator::new()
        .alias("optional_email", "omitempty,email")?
        .value(&String::new(), "optional_email,required").await?;

    Ok(())
}

#[tokio::test]
async fn alias_omitempty_in_alternative_skips_following_rules() -> Result<(), Box<dyn std::error::Error>>
{
    Validator::new()
        .alias("optional_email", "omitempty,email")?
        .value(&String::new(), "email|optional_email,required").await?;

    Ok(())
}

#[derive(Debug, Validate)]
struct OptionalAliasEmail {
    #[validate(alias = "optional_email", required)]
    email: String,
}

#[tokio::test]
async fn derive_alias_omitempty_skips_following_rules() -> Result<(), Box<dyn std::error::Error>> {
    Validator::new()
        .alias("optional_email", "omitempty,email")?
        .validate(&OptionalAliasEmail {
            email: String::new(),
        }).await?;

    Ok(())
}

#[tokio::test]
async fn schema_alias_omitempty_skips_following_rules() -> Result<(), Box<dyn std::error::Error>> {
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
        .check_map(&serde_json::json!({ "email": "" })).await?;

    Ok(())
}

#[tokio::test]
async fn direct_value_alternatives_preserve_joined_reason() {
    let fields = fields(
        Validator::new()
            .value(&"not-a-color", "hexcolor|rgb|rgba").await
            .unwrap_err(),
    );

    assert_eq!(fields.len(), 1);
    assert_eq!(fields[0].rule(), "hexcolor|rgb|rgba");
    assert_eq!(fields[0].reason(), "hexcolor|rgb|rgba");
}

#[tokio::test]
async fn direct_value_reuses_alias_and_custom_rule() -> Result<(), Box<dyn std::error::Error>> {
    Validator::new()
        .alias("slug_alias", "slug")?
        .rule("slug", Slug)?
        .value(&"hello-rust", "slug_alias").await?;

    Ok(())
}

#[tokio::test]
async fn direct_value_rejects_rule_alias_name_collision() -> Result<(), Box<dyn std::error::Error>> {
    let validator = Validator::new().alias("slug", "required")?;
    assert!(validator.value(&String::new(), "slug").await.is_err());

    let Err(error) = validator.rule("slug", Slug) else {
        panic!("expected duplicate name error");
    };
    assert!(matches!(error, Error::DuplicateName { name } if name == "slug"));

    Ok(())
}

#[tokio::test]
async fn direct_value_invalid_expression_returns_error() {
    let error = Validator::new().value(&"abc", "length(min=3").await.unwrap_err();

    assert!(matches!(
        error,
        halo::validator::Error::InvalidRuleExpression { .. }
    ));
}

#[tokio::test]
async fn direct_value_unknown_rule_returns_error() {
    let error = Validator::new().value(&"abc", "missing_rule").await.unwrap_err();

    assert!(matches!(
        error,
        halo::validator::Error::UnknownRule { name } if name == "missing_rule"
    ));
}

#[tokio::test]
async fn direct_value_alias_with_unknown_rule_returns_error() -> Result<(), Box<dyn std::error::Error>> {
    let error = Validator::new()
        .alias("broken_alias", "missing_rule")?
        .value(&"abc", "broken_alias").await
        .unwrap_err();

    assert!(matches!(
        error,
        halo::validator::Error::UnknownRule { name } if name == "missing_rule"
    ));

    Ok(())
}
