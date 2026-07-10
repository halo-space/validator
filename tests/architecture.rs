use serde_json::json;
use validator::prelude::*;

#[derive(Debug)]
struct Slug;

impl Rule for Slug {
    fn check(&self, field: &Field<'_>) -> Result<bool, Error> {
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

    fn check(&self, field: &Field<'_>) -> Result<bool, Error> {
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

#[test]
fn custom_marker_rule_uses_the_same_contract_in_every_entry() {
    let derive = Validator::new()
        .rule("slug", Slug)
        .unwrap()
        .validate(&Post {
            slug: "Hello World".to_owned(),
        })
        .unwrap_err()
        .into_fields()
        .unwrap();

    let direct = Validator::new()
        .rule("slug", Slug)
        .unwrap()
        .value(&"Hello World", "slug")
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
        .validate_map(&json!({ "slug": "Hello World" }))
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

#[test]
fn custom_parameter_rule_uses_the_same_contract_in_every_entry() {
    let derive = Validator::new()
        .rule("starts_with", StartsWith)
        .unwrap()
        .validate(&Article {
            slug: "entry-1".to_owned(),
        })
        .unwrap_err()
        .into_fields()
        .unwrap();

    let direct = Validator::new()
        .rule("starts_with", StartsWith)
        .unwrap()
        .value(&"entry-1", "starts_with=post-")
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
        .validate_map(&json!({ "slug": "entry-1" }))
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

#[test]
fn signatures_reject_unknown_missing_and_extra_parameters() {
    for expression in ["min(mian=3)", "min", "email(foo=bar)"] {
        let error = Validator::new().value(&"rust", expression).unwrap_err();
        assert!(matches!(error, Error::InvalidRuleExpression { .. }));
    }

    let derive = Validator::new()
        .validate(&InvalidLength {
            value: "rust".to_owned(),
        })
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
        .validate_map(&json!({ "value": "rust" }))
        .unwrap_err();
    assert!(matches!(schema, Error::InvalidRuleExpression { .. }));
}

#[test]
fn choice_values_keep_delimiter_characters() {
    Validator::new()
        .value(&"a,b", r#"oneof("a,b","c")"#)
        .unwrap();
    Validator::new()
        .value(&"a=b", r#"oneof("a=b","c")"#)
        .unwrap();

    let fields = Validator::new()
        .value(&"other", r#"oneof("a,b","c")"#)
        .unwrap_err()
        .into_fields()
        .unwrap();
    let values = fields[0].params().list("values").unwrap();

    assert_eq!(values, &["a,b", "c"]);
}

#[test]
fn field_rule_requires_field_context() {
    let error = Validator::new()
        .value(&"value", "eq_field=other")
        .unwrap_err();

    assert!(matches!(
        error,
        Error::MissingFieldContext { name } if name == "eq_field"
    ));
}

#[test]
fn recursive_aliases_are_rejected() {
    let direct = Validator::new()
        .alias("cycle", "cycle")
        .unwrap()
        .value(&"value", "cycle")
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
        .unwrap_err();
    assert!(matches!(
        indirect,
        Error::RecursiveAlias { name } if name == "first"
    ));
}

#[test]
fn built_in_and_control_names_cannot_be_overridden() {
    for name in ["required", "omitempty"] {
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
