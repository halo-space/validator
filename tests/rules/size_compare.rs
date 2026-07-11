use std::collections::{BTreeMap, HashMap};

use validator::prelude::*;

fn param_list<'a>(field: &'a FieldError, name: &str) -> Vec<&'a str> {
    field
        .params()
        .list(name)
        .expect("expected list parameter")
        .iter()
        .map(String::as_str)
        .collect()
}

#[derive(Debug, Validate)]
struct Bounds {
    #[validate(min = 3)]
    name: String,

    #[validate(max = 2)]
    tags: Vec<String>,

    #[validate(range(min = 10, max = 20))]
    score: u32,
}

#[test]
fn min_max_range_pass() {
    let value = Bounds {
        name: "rust".to_owned(),
        tags: vec!["a".to_owned(), "b".to_owned()],
        score: 15,
    };

    Validator::new().validate(&value).unwrap();
}

#[test]
fn min_max_range_fail() {
    let value = Bounds {
        name: "rs".to_owned(),
        tags: vec!["a".to_owned(), "b".to_owned(), "c".to_owned()],
        score: 30,
    };

    let fields = Validator::new()
        .validate(&value)
        .unwrap_err()
        .into_fields()
        .unwrap();

    assert_eq!(fields.len(), 3);
    assert_eq!(fields[0].rule(), "min");
    assert_eq!(fields[0].params().text("min"), Some("3"));
    assert_eq!(fields[1].rule(), "max");
    assert_eq!(fields[1].params().text("max"), Some("2"));
    assert_eq!(fields[2].rule(), "range");
    assert_eq!(fields[2].params().text("min"), Some("10"));
    assert_eq!(fields[2].params().text("max"), Some("20"));
}

#[test]
fn size_rules_reject_conflicting_or_reversed_bounds() {
    let validator = Validator::new();

    for error in [
        validator.value(&"", "length").unwrap_err(),
        validator
            .value(&"", "omitempty,length(exact=3,min=1)")
            .unwrap_err(),
        validator
            .value(&"", "omitempty,length(min=3,max=1)")
            .unwrap_err(),
        validator.value(&5_i32, "range(min=10,max=1)").unwrap_err(),
    ] {
        assert!(matches!(error, Error::InvalidRuleExpression { .. }));
    }
}

#[derive(Debug, Validate)]
struct Comparisons {
    #[validate(gte = 0, lte = 130)]
    age: u8,

    #[validate(gte = 3, lte = 5)]
    name: String,

    #[validate(gt = -10, lt = 10)]
    score: i32,

    #[validate(gt = 1.5, lt = 3.5)]
    ratio: f32,

    #[validate(lte = 2)]
    tags: Vec<String>,
}

#[test]
fn comparison_rules_dispatch_by_field_type() {
    let value = Comparisons {
        age: 42,
        name: "rust".to_owned(),
        score: -9,
        ratio: 2.5,
        tags: vec!["a".to_owned(), "b".to_owned()],
    };

    Validator::new().validate(&value).unwrap();
}

#[test]
fn comparison_rules_report_type_specific_failures() {
    let value = Comparisons {
        age: 131,
        name: "rs".to_owned(),
        score: -10,
        ratio: 3.5,
        tags: vec!["a".to_owned(), "b".to_owned(), "c".to_owned()],
    };

    let fields = Validator::new()
        .validate(&value)
        .unwrap_err()
        .into_fields()
        .unwrap();
    let rules = fields.iter().map(|field| field.rule()).collect::<Vec<_>>();

    assert_eq!(rules, vec!["lte", "gte", "gt", "lt", "lte"]);
    assert_eq!(fields[0].params().text("value"), Some("130"));
    assert_eq!(fields[2].params().text("value"), Some("-10"));
    assert_eq!(fields[3].params().text("value"), Some("3.5"));
}

#[derive(Debug, Validate)]
struct Defaults {
    #[validate(isdefault)]
    name: String,

    #[validate(isdefault)]
    count: u32,

    #[validate(isdefault)]
    enabled: bool,

    #[validate(isdefault)]
    tags: Vec<String>,
}

#[test]
fn isdefault_accepts_default_values() {
    let value = Defaults {
        name: String::new(),
        count: 0,
        enabled: false,
        tags: Vec::new(),
    };

    Validator::new().validate(&value).unwrap();
    Validator::new().value(&0_u32, "isdefault").unwrap();
}

#[test]
fn isdefault_rejects_non_default_values() {
    let value = Defaults {
        name: "rust".to_owned(),
        count: 1,
        enabled: true,
        tags: vec!["rust".to_owned()],
    };

    let fields = Validator::new()
        .validate(&value)
        .unwrap_err()
        .into_fields()
        .unwrap();
    let rules = fields.iter().map(|field| field.rule()).collect::<Vec<_>>();

    assert_eq!(
        rules,
        vec!["isdefault", "isdefault", "isdefault", "isdefault"]
    );
}
