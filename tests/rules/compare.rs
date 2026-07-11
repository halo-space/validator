
#[derive(Debug, Validate)]
struct EqualityRules {
    #[validate(eq = "published")]
    state: String,

    #[validate(ne = 0)]
    score: i32,

    #[validate(eq = true)]
    active: bool,

    #[validate(eq = 2)]
    tags: Vec<String>,
}

#[test]
fn equality_rules_pass() {
    let value = EqualityRules {
        state: "published".to_owned(),
        score: 42,
        active: true,
        tags: vec!["rust".to_owned(), "validator".to_owned()],
    };

    Validator::new().validate(&value).unwrap();
}

#[test]
fn equality_rules_fail() {
    let value = EqualityRules {
        state: "draft".to_owned(),
        score: 0,
        active: false,
        tags: vec!["rust".to_owned()],
    };

    let fields = Validator::new()
        .validate(&value)
        .unwrap_err()
        .into_fields()
        .unwrap();
    let rules = fields.iter().map(|field| field.rule()).collect::<Vec<_>>();

    assert_eq!(rules, vec!["eq", "ne", "eq", "eq"]);
    assert_eq!(fields[0].params().text("value"), Some("published"));
    assert_eq!(fields[1].params().text("value"), Some("0"));
    assert_eq!(fields[2].params().text("value"), Some("true"));
    assert_eq!(fields[3].params().text("value"), Some("2"));
}
