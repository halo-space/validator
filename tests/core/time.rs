#[test]
fn system_time_compares_with_captured_now() {
    let now = SystemTime::now();

    Validator::new()
        .validate(&TimeEvent {
            created_at: now
                .checked_sub(Duration::from_secs(60))
                .expect("current time should support subtracting one minute"),
            expires_at: now + Duration::from_secs(60),
        })
        .unwrap();

    let fields = fields(
        Validator::new()
            .validate(&TimeEvent {
                created_at: now + Duration::from_secs(60),
                expires_at: now
                    .checked_sub(Duration::from_secs(60))
                    .expect("current time should support subtracting one minute"),
            })
            .unwrap_err(),
    );
    let rules = fields.iter().map(|field| field.rule()).collect::<Vec<_>>();

    assert_eq!(rules, vec!["lte", "gt"]);
    assert_eq!(fields[0].kind(), Kind::Time);
    assert_eq!(fields[1].kind(), Kind::Time);
}

#[test]
fn direct_system_time_value_uses_now_rules() -> Result<(), Box<dyn std::error::Error>> {
    let now = SystemTime::now();
    let past = now
        .checked_sub(Duration::from_secs(60))
        .expect("current time should support subtracting one minute");
    let future = now + Duration::from_secs(60);

    Validator::new().value(&past, "lte")?;
    Validator::new().value(&future, "gt")?;

    Ok(())
}

#[derive(Debug, Validate)]
struct TimeWindow {
    created_at: SystemTime,

    #[validate(gt_field = "created_at")]
    updated_at: SystemTime,

    #[validate(eq_field = "created_at")]
    copied_at: SystemTime,
}

#[test]
fn system_time_cross_field_rules_compare_time_values() {
    let created_at = SystemTime::now();

    Validator::new()
        .validate(&TimeWindow {
            created_at,
            updated_at: created_at + Duration::from_secs(60),
            copied_at: created_at,
        })
        .unwrap();

    let fields = fields(
        Validator::new()
            .validate(&TimeWindow {
                created_at,
                updated_at: created_at,
                copied_at: created_at + Duration::from_secs(1),
            })
            .unwrap_err(),
    );
    let rules = fields.iter().map(|field| field.rule()).collect::<Vec<_>>();

    assert_eq!(rules, vec!["gt_field", "eq_field"]);
}

#[derive(Debug, Validate)]
struct TimeStrictKind {
    created_at_text: String,

    #[validate(gt_field = "created_at_text")]
    updated_at: SystemTime,
}

#[test]
fn system_time_cross_field_requires_time_kind() {
    let fields = fields(
        Validator::new()
            .validate(&TimeStrictKind {
                created_at_text: "2026-07-08T00:00:00Z".to_owned(),
                updated_at: SystemTime::now(),
            })
            .unwrap_err(),
    );

    assert_eq!(fields.len(), 1);
    assert_eq!(fields[0].rule(), "gt_field");
}

#[derive(Debug, Validate)]
struct ParameterizedTime {
    #[validate(gt = "2026-07-08T00:00:00Z")]
    at: SystemTime,
}

#[test]
fn parameterized_system_time_comparison_returns_config_error() {
    let error = Validator::new()
        .validate(&ParameterizedTime {
            at: SystemTime::now(),
        })
        .unwrap_err();

    assert!(matches!(
        error,
        Error::InvalidRuleExpression { reason, .. }
            if reason.contains("SystemTime comparison does not support literal parameters")
    ));
}

#[derive(Debug, Validate)]
struct EqualNowTime {
    #[validate(eq)]
    at: SystemTime,
}

#[test]
fn system_time_eq_now_returns_config_error() {
    let error = Validator::new()
        .validate(&EqualNowTime {
            at: SystemTime::now(),
        })
        .unwrap_err();

    assert!(matches!(
        error,
        Error::InvalidRuleExpression { reason, .. }
            if reason.contains("SystemTime eq/ne against the current time is unsupported")
    ));
}

#[test]
fn direct_system_time_parameter_returns_config_error() {
    let error = Validator::new()
        .value(&SystemTime::now(), r#"gt(value="2026-07-08T00:00:00Z")"#)
        .unwrap_err();

    assert!(matches!(
        error,
        Error::InvalidRuleExpression { reason, .. }
            if reason.contains("SystemTime comparison does not support literal parameters")
    ));
}

#[test]
fn direct_alternative_system_time_config_error_is_not_swallowed() {
    let error = Validator::new()
        .value(
            &SystemTime::now(),
            r#"gt(value="2026-07-08T00:00:00Z")|lte"#,
        )
        .unwrap_err();

    assert!(matches!(
        error,
        Error::InvalidRuleExpression { reason, .. }
            if reason.contains("SystemTime comparison does not support literal parameters")
    ));
}

#[test]
fn schema_rejects_native_time_type() {
    let error = Schema::from_yaml(
        r#"
fields:
  created_at:
    type: time
"#,
    )
    .unwrap_err();

    assert!(matches!(
        error,
        Error::InvalidSchema { reason } if reason.contains("unsupported type 'time'")
    ));
}
