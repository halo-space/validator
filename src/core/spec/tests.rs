use super::{Expr, parse_expression};

#[test]
fn parses_rule_expression() {
    let exprs = parse_expression("required,length(min=3,max=20)").unwrap();

    assert_eq!(exprs.len(), 2);
    let required = exprs[0].single().unwrap();
    let length = exprs[1].single().unwrap();

    assert_eq!(required.name(), "required");
    assert_eq!(length.name(), "length");
    assert_eq!(length.params().named_values().len(), 2);
}

#[test]
fn parses_oneof_with_quoted_values() {
    let exprs = parse_expression(r#"oneof("draft","published")"#).unwrap();
    let spec = exprs[0].single().unwrap();

    assert_eq!(exprs.len(), 1);
    assert_eq!(spec.name(), "oneof");
    assert_eq!(spec.params().positional_values(), ["draft", "published"]);
}

#[test]
fn parses_space_separated_equal_params() {
    let exprs = parse_expression("unique=tenant_id profile.email").unwrap();
    let spec = exprs[0].single().unwrap();

    assert_eq!(
        spec.params().positional_values(),
        ["tenant_id", "profile.email"]
    );

    let exprs = parse_expression(r#"eq="hello world""#).unwrap();
    let spec = exprs[0].single().unwrap();
    assert_eq!(spec.params().positional_values(), ["hello world"]);
}

#[test]
fn parses_conditional_field_lists() {
    let exprs = parse_expression(r#"required_with("email","phone")"#).unwrap();
    let spec = exprs[0].single().unwrap();

    assert_eq!(exprs.len(), 1);
    assert_eq!(spec.name(), "required_with");
    assert_eq!(spec.params().positional_values(), ["email", "phone"]);

    let exprs = parse_expression("required_without=email").unwrap();
    let spec = exprs[0].single().unwrap();

    assert_eq!(exprs.len(), 1);
    assert_eq!(spec.name(), "required_without");
    assert_eq!(spec.params().positional_values(), ["email"]);

    let exprs = parse_expression(r#"excluded_with_all("email","phone")"#).unwrap();
    let spec = exprs[0].single().unwrap();

    assert_eq!(exprs.len(), 1);
    assert_eq!(spec.name(), "excluded_with_all");
    assert_eq!(spec.params().positional_values(), ["email", "phone"]);

    let exprs = parse_expression("required_without_all=email").unwrap();
    let spec = exprs[0].single().unwrap();

    assert_eq!(exprs.len(), 1);
    assert_eq!(spec.name(), "required_without_all");
    assert_eq!(spec.params().positional_values(), ["email"]);
}

#[test]
fn parses_rule_alternatives() {
    let exprs = parse_expression("required,hexcolor|rgb|rgba").unwrap();

    assert_eq!(exprs.len(), 2);
    assert!(matches!(exprs[0], Expr::Single(_)));

    let alternatives = exprs[1].alternatives().unwrap();
    let names = alternatives
        .iter()
        .map(|spec| spec.name())
        .collect::<Vec<_>>();

    assert_eq!(names, vec!["hexcolor", "rgb", "rgba"]);
}

#[test]
fn rejects_empty_and_unbalanced_expressions() {
    for expression in ["", "email,", "email||url", "oneof(\"value)", "email)"] {
        assert!(parse_expression(expression).is_err(), "{expression}");
    }
}

#[test]
fn decodes_escaped_quoted_parameters() {
    let exprs = parse_expression(r#"oneof("a\"b","c\\d")"#).unwrap();
    let values = exprs[0].single().unwrap().params().positional_values();

    assert_eq!(values, ["a\"b", "c\\d"]);
}

#[test]
fn quoted_delimiters_remain_parameter_values() {
    for (expression, expected) in [
        (r#"eq="(""#, "("),
        (r#"eq=")""#, ")"),
        (r#"eq=",""#, ","),
        (r#"eq="|""#, "|"),
    ] {
        let exprs = parse_expression(expression).unwrap();
        let values = exprs[0].single().unwrap().params().positional_values();
        assert_eq!(values, [expected], "{expression}");
    }
}
