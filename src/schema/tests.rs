use super::Schema;

#[test]
fn cloned_schema_keeps_identity() {
    let schema = Schema::from_yaml(
        r#"
fields:
  title:
    type: string
"#,
    )
    .unwrap();
    let cloned = schema.clone();

    assert_eq!(schema.id(), cloned.id());
}

#[test]
fn equivalent_schemas_share_identity_for_compilation_cache_reuse() {
    let first = Schema::from_json(r#"{"fields":{"title":{"type":"string"}}}"#).unwrap();
    let second = Schema::from_yaml(
        r#"
fields:
  title:
    type: string
"#,
    )
    .unwrap();

    assert_eq!(first.id(), second.id());
}

#[test]
fn scalar_schema_field_rejects_nested_fields() {
    let error = Schema::from_yaml(
        r#"
fields:
  title:
    type: string
    fields:
      value:
        type: string
"#,
    )
    .unwrap_err();

    assert!(matches!(error, crate::Error::InvalidSchema { .. }));
}
