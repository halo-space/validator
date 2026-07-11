
#[derive(Debug, Validate)]
struct OptionalFormats {
    #[validate(omitempty, email)]
    email: String,

    #[validate(omitempty, min = 3)]
    nickname: String,

    #[validate(omitempty, gte = 10)]
    score: u32,
}

#[test]
fn omitempty_skips_following_rules_for_empty_values() {
    let value = OptionalFormats {
        email: String::new(),
        nickname: String::new(),
        score: 0,
    };

    Validator::new().validate(&value).unwrap();
}

#[test]
fn omitempty_validates_following_rules_for_non_empty_values() {
    let value = OptionalFormats {
        email: "not-email".to_owned(),
        nickname: "rs".to_owned(),
        score: 5,
    };

    let fields = Validator::new()
        .validate(&value)
        .unwrap_err()
        .into_fields()
        .unwrap();
    let rules = fields.iter().map(|field| field.rule()).collect::<Vec<_>>();

    assert_eq!(rules, vec!["email", "min", "gte"]);
}

#[derive(Debug, Validate)]
struct AliasOptionalEmail {
    #[validate(alias = "optional_email")]
    email: String,
}

#[test]
fn omitempty_works_inside_alias_expression() -> Result<(), Box<dyn std::error::Error>> {
    let empty = AliasOptionalEmail {
        email: String::new(),
    };
    Validator::new()
        .alias("optional_email", "omitempty,email")?
        .validate(&empty)?;

    let invalid = AliasOptionalEmail {
        email: "not-email".to_owned(),
    };
    let fields = Validator::new()
        .alias("optional_email", "omitempty,email")?
        .validate(&invalid)
        .unwrap_err()
        .into_fields()
        .unwrap();

    assert_eq!(fields.len(), 1);
    assert_eq!(fields[0].rule(), "optional_email");
    assert_eq!(fields[0].reason(), "email");

    Ok(())
}
