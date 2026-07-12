
#[derive(Debug, Validate)]
struct UniqueCollections<'a> {
    #[validate(unique)]
    tags: Vec<String>,

    #[validate(unique)]
    scores: [u8; 3],

    #[validate(unique)]
    aliases: &'a [&'a str],

    #[validate(unique)]
    labels: HashMap<String, String>,

    #[validate(unique)]
    metadata: BTreeMap<String, u8>,
}

#[test]
fn unique_collections_pass() {
    let aliases = ["validator", "rules"];
    let value = UniqueCollections {
        tags: vec!["rust".to_owned(), "validator".to_owned()],
        scores: [1, 2, 3],
        aliases: &aliases,
        labels: HashMap::from([
            ("a".to_owned(), "rust".to_owned()),
            ("b".to_owned(), "rules".to_owned()),
        ]),
        metadata: BTreeMap::from([("a".to_owned(), 1), ("b".to_owned(), 2)]),
    };

    Validator::new().validate(&value).unwrap();
}

#[test]
fn unique_collections_fail_on_field_namespace() {
    let aliases = ["validator", "validator"];
    let value = UniqueCollections {
        tags: vec!["rust".to_owned(), "rust".to_owned()],
        scores: [1, 2, 1],
        aliases: &aliases,
        labels: HashMap::from([
            ("a".to_owned(), "rust".to_owned()),
            ("b".to_owned(), "rust".to_owned()),
        ]),
        metadata: BTreeMap::from([("a".to_owned(), 1), ("b".to_owned(), 1)]),
    };

    let fields = Validator::new()
        .validate(&value)
        .unwrap_err()
        .into_fields()
        .unwrap();
    let failures = fields
        .iter()
        .map(|field| (field.namespace().as_str(), field.rule()))
        .collect::<Vec<_>>();

    assert_eq!(
        failures,
        vec![
            ("UniqueCollections.tags", "unique"),
            ("UniqueCollections.scores", "unique"),
            ("UniqueCollections.aliases", "unique"),
            ("UniqueCollections.labels", "unique"),
            ("UniqueCollections.metadata", "unique"),
        ]
    );
}

#[test]
fn direct_json_array_unique_rule_works() {
    Validator::new()
        .value(&serde_json::json!(["rust", "validator"]), "unique")
        .unwrap();

    let fields = Validator::new()
        .value(&serde_json::json!(["rust", "rust"]), "unique")
        .unwrap_err()
        .into_fields()
        .unwrap();

    assert_eq!(fields[0].namespace().as_str(), "$value");
    assert_eq!(fields[0].rule(), "unique");
}

#[test]
fn direct_json_unique_preserves_scalar_families_and_float_zero() {
    let validator = Validator::new();

    validator
        .value(&serde_json::json!([1, "1", true, null]), "unique")
        .unwrap();

    for value in [
        serde_json::json!([true, true]),
        serde_json::json!([0.0, -0.0]),
        serde_json::json!([null, null]),
    ] {
        let fields = validator
            .value(&value, "unique")
            .unwrap_err()
            .into_fields()
            .unwrap();

        assert_eq!(fields[0].rule(), "unique");
    }
}

#[test]
fn direct_json_object_unique_rule_checks_values() {
    Validator::new()
        .value(
            &serde_json::json!({ "a": "rust", "b": "validator" }),
            "unique",
        )
        .unwrap();

    let fields = Validator::new()
        .value(&serde_json::json!({ "a": "rust", "b": "rust" }), "unique")
        .unwrap_err()
        .into_fields()
        .unwrap();

    assert_eq!(fields[0].namespace().as_str(), "$value");
    assert_eq!(fields[0].rule(), "unique");
}

#[test]
fn direct_native_collections_unique_rule_works() {
    let validator = Validator::new();
    let array = ["rust", "validator"];
    let slice = &array[..];
    let hash_map = HashMap::from([("a", "rust"), ("b", "validator")]);
    let btree_map = BTreeMap::from([("a", 1_u8), ("b", 2_u8)]);

    validator
        .value(&vec!["rust", "validator"], "unique")
        .unwrap();
    validator.value(&array, "unique").unwrap();
    validator.value(&slice, "unique").unwrap();
    validator.value(&hash_map, "unique").unwrap();
    validator.value(&btree_map, "unique").unwrap();

    for fields in [
        validator
            .value(&vec!["rust", "rust"], "unique")
            .unwrap_err()
            .into_fields()
            .unwrap(),
        validator
            .value(&[1_u8, 1_u8], "unique")
            .unwrap_err()
            .into_fields()
            .unwrap(),
        validator
            .value(&HashMap::from([("a", 1_u8), ("b", 1_u8)]), "unique")
            .unwrap_err()
            .into_fields()
            .unwrap(),
    ] {
        assert_eq!(fields[0].namespace().as_str(), "$value");
        assert_eq!(fields[0].rule(), "unique");
    }
}

#[test]
fn unique_rejects_unsupported_json_array_elements() {
    let error = Validator::new()
        .value(&serde_json::json!([{ "id": 1 }]), "unique")
        .unwrap_err();

    assert!(matches!(error, Error::InvalidRuleExpression { .. }));
}

#[test]
fn unique_preflight_rejects_scalar_behind_omitempty() {
    let error = Validator::new()
        .value(&0_i32, "omitempty,unique")
        .unwrap_err();

    assert!(matches!(error, Error::InvalidRuleExpression { .. }));
}

#[test]
fn unique_treats_nan_values_as_distinct() {
    Validator::new()
        .value(&vec![f64::NAN, f64::NAN, 1.0], "unique")
        .unwrap();
}

#[derive(Debug)]
struct UniqueAccount {
    email: String,
    age: u32,
    nickname: Option<String>,
    created_at: std::time::SystemTime,
}

#[derive(Debug, Validate)]
struct UniqueAccountVec {
    #[validate(unique = "email")]
    users: Vec<UniqueAccount>,
}

#[derive(Debug, Validate)]
struct UniqueAccountArray {
    #[validate(unique = "age")]
    users: [UniqueAccount; 2],
}

#[derive(Debug, Validate)]
struct UniqueAccountSlice<'a> {
    #[validate(unique = "created_at")]
    users: &'a [UniqueAccount],
}

#[derive(Debug, Validate)]
struct UniqueOptionalField {
    #[validate(unique = "nickname")]
    users: Vec<UniqueAccount>,
}

#[derive(Debug)]
struct UnsupportedUniqueKey;

impl Value for UnsupportedUniqueKey {
    fn kind(&self) -> Kind {
        Kind::Other
    }

    fn required(&self) -> bool {
        true
    }
}

#[derive(Debug)]
struct UniqueUnsupportedItem {
    value: UnsupportedUniqueKey,
}

#[derive(Debug, Validate)]
struct UniqueUnsupportedField {
    #[validate(unique = "value")]
    items: Vec<UniqueUnsupportedItem>,
}

#[derive(Debug)]
struct UniqueScalarItem {
    enabled: bool,
    balance: i64,
    ratio: f64,
}

#[derive(Debug, Validate)]
struct UniqueScalarFields {
    #[validate(unique = "enabled")]
    bools: Vec<UniqueScalarItem>,
    #[validate(unique = "balance")]
    ints: Vec<UniqueScalarItem>,
    #[validate(unique = "ratio")]
    floats: Vec<UniqueScalarItem>,
}

#[derive(Debug)]
struct RawUniqueItem {
    r#type: String,
}

#[derive(Debug, Validate)]
struct RawUniqueItems {
    #[validate(unique = "type")]
    items: Vec<RawUniqueItem>,
}

#[derive(Debug)]
struct UniqueProfile {
    email: String,
    score: f64,
}

#[derive(Debug)]
struct UniqueCompoundItem {
    tenant_id: u64,
    profile: Option<UniqueProfile>,
    created_at: std::time::SystemTime,
}

#[derive(Debug, Validate)]
struct UniqueCompoundFields {
    #[validate(unique = ["tenant_id", "profile.email"])]
    users: Vec<UniqueCompoundItem>,
}

#[derive(Debug, Validate)]
struct UniqueCompoundTime {
    #[validate(unique = ["tenant_id", "created_at"])]
    users: Vec<UniqueCompoundItem>,
}

#[derive(Debug, Validate)]
struct UniqueCompoundFloat {
    #[validate(unique = ["tenant_id", "profile.score"])]
    users: Vec<UniqueCompoundItem>,
}

#[derive(Debug)]
struct WideUniqueItem {
    first: u8,
    second: u8,
    third: u8,
    fourth: u8,
    fifth: u8,
}

#[derive(Debug, Validate)]
struct WideUniqueFields {
    #[validate(unique = ["first", "second", "third", "fourth", "fifth"])]
    items: Vec<WideUniqueItem>,
}

fn unique_account(email: &str, age: u32, nickname: Option<&str>, second: u64) -> UniqueAccount {
    UniqueAccount {
        email: email.to_owned(),
        age,
        nickname: nickname.map(str::to_owned),
        created_at: std::time::UNIX_EPOCH + std::time::Duration::from_secs(second),
    }
}

fn compound_item(
    tenant_id: u64,
    email: Option<&str>,
    score: f64,
    second: u64,
) -> UniqueCompoundItem {
    UniqueCompoundItem {
        tenant_id,
        profile: email.map(|email| UniqueProfile {
            email: email.to_owned(),
            score,
        }),
        created_at: std::time::UNIX_EPOCH + std::time::Duration::from_secs(second),
    }
}

#[test]
fn unique_field_supports_vec_array_and_slice() {
    Validator::new()
        .validate(&UniqueAccountVec {
            users: vec![
                unique_account("first@example.com", 1, Some("first"), 1),
                unique_account("second@example.com", 2, Some("second"), 2),
            ],
        })
        .unwrap();

    Validator::new()
        .validate(&UniqueAccountArray {
            users: [
                unique_account("same@example.com", 1, Some("first"), 1),
                unique_account("same@example.com", 2, Some("second"), 2),
            ],
        })
        .unwrap();

    let users = [
        unique_account("same@example.com", 1, Some("first"), 1),
        unique_account("same@example.com", 1, Some("second"), 2),
    ];
    Validator::new()
        .validate(&UniqueAccountSlice { users: &users })
        .unwrap();
}

#[test]
fn unique_field_supports_raw_identifier_members() {
    let fields = Validator::new()
        .validate(&RawUniqueItems {
            items: vec![
                RawUniqueItem {
                    r#type: "same".to_owned(),
                },
                RawUniqueItem {
                    r#type: "same".to_owned(),
                },
            ],
        })
        .unwrap_err()
        .into_fields()
        .unwrap();

    assert_eq!(fields.len(), 1);
    assert_eq!(param_list(&fields[0], "fields"), vec!["type"]);
}

#[test]
fn unique_compound_fields_compare_complete_nested_keys() {
    Validator::new()
        .validate(&UniqueCompoundFields {
            users: vec![
                compound_item(1, Some("same@example.com"), 1.0, 1),
                compound_item(2, Some("same@example.com"), 1.0, 2),
            ],
        })
        .unwrap();

    let fields = Validator::new()
        .validate(&UniqueCompoundFields {
            users: vec![
                compound_item(1, Some("same@example.com"), 1.0, 1),
                compound_item(1, Some("same@example.com"), 2.0, 2),
            ],
        })
        .unwrap_err()
        .into_fields()
        .unwrap();

    assert_eq!(fields.len(), 1);
    assert_eq!(fields[0].namespace().as_str(), "UniqueCompoundFields.users");
    assert_eq!(fields[0].rule(), "unique");
    assert_eq!(
        param_list(&fields[0], "fields"),
        vec!["tenant_id", "profile.email"]
    );
    assert_eq!(fields[0].params().text("field"), None);
}

#[test]
fn unique_compound_fields_support_more_than_inline_capacity() {
    let item = |fifth| WideUniqueItem {
        first: 1,
        second: 2,
        third: 3,
        fourth: 4,
        fifth,
    };

    Validator::new()
        .validate(&WideUniqueFields {
            items: vec![item(5), item(6)],
        })
        .unwrap();

    let fields = Validator::new()
        .validate(&WideUniqueFields {
            items: vec![item(5), item(5)],
        })
        .unwrap_err()
        .into_fields()
        .unwrap();

    assert_eq!(fields.len(), 1);
    assert_eq!(fields[0].rule(), "unique");
    assert_eq!(
        param_list(&fields[0], "fields"),
        vec!["first", "second", "third", "fourth", "fifth"]
    );
}

#[test]
fn unique_compound_fields_preserve_none_time_and_nan_semantics() {
    Validator::new()
        .validate(&UniqueCompoundFields {
            users: vec![
                compound_item(1, None, 0.0, 1),
                compound_item(2, None, 0.0, 2),
            ],
        })
        .unwrap();

    let none = Validator::new()
        .validate(&UniqueCompoundFields {
            users: vec![
                compound_item(1, None, 0.0, 1),
                compound_item(1, None, 0.0, 2),
            ],
        })
        .unwrap_err()
        .into_fields()
        .unwrap();
    assert_eq!(
        param_list(&none[0], "fields"),
        vec!["tenant_id", "profile.email"]
    );

    Validator::new()
        .validate(&UniqueCompoundTime {
            users: vec![
                compound_item(1, Some("first@example.com"), 1.0, 1),
                compound_item(1, Some("second@example.com"), 2.0, 2),
            ],
        })
        .unwrap();
    let time = Validator::new()
        .validate(&UniqueCompoundTime {
            users: vec![
                compound_item(1, Some("first@example.com"), 1.0, 1),
                compound_item(1, Some("second@example.com"), 2.0, 1),
            ],
        })
        .unwrap_err()
        .into_fields()
        .unwrap();
    assert_eq!(
        param_list(&time[0], "fields"),
        vec!["tenant_id", "created_at"]
    );

    Validator::new()
        .validate(&UniqueCompoundFloat {
            users: vec![
                compound_item(1, Some("first@example.com"), f64::NAN, 1),
                compound_item(1, Some("second@example.com"), f64::NAN, 2),
            ],
        })
        .unwrap();
}

#[test]
fn unique_field_reports_collection_error_and_params() {
    let fields = Validator::new()
        .validate(&UniqueAccountVec {
            users: vec![
                unique_account("same@example.com", 1, Some("first"), 1),
                unique_account("same@example.com", 2, Some("second"), 2),
            ],
        })
        .unwrap_err()
        .into_fields()
        .unwrap();

    assert_eq!(fields.len(), 1);
    assert_eq!(fields[0].namespace().as_str(), "UniqueAccountVec.users");
    assert_eq!(fields[0].rule(), "unique");
    assert_eq!(fields[0].reason(), "unique");
    assert_eq!(param_list(&fields[0], "fields"), vec!["email"]);
}

#[test]
fn unique_field_treats_none_as_one_key() {
    Validator::new()
        .validate(&UniqueOptionalField {
            users: vec![
                unique_account("first@example.com", 1, None, 1),
                unique_account("second@example.com", 2, Some("name"), 2),
            ],
        })
        .unwrap();

    let fields = Validator::new()
        .validate(&UniqueOptionalField {
            users: vec![
                unique_account("first@example.com", 1, None, 1),
                unique_account("second@example.com", 2, None, 2),
            ],
        })
        .unwrap_err()
        .into_fields()
        .unwrap();
    assert_eq!(fields.len(), 1);
    assert_eq!(param_list(&fields[0], "fields"), vec!["nickname"]);
}

#[test]
fn unique_field_rejects_duplicate_numeric_and_time_keys() {
    let numeric = Validator::new()
        .validate(&UniqueAccountArray {
            users: [
                unique_account("first@example.com", 1, Some("first"), 1),
                unique_account("second@example.com", 1, Some("second"), 2),
            ],
        })
        .unwrap_err()
        .into_fields()
        .unwrap();
    assert_eq!(param_list(&numeric[0], "fields"), vec!["age"]);

    let users = [
        unique_account("first@example.com", 1, Some("first"), 1),
        unique_account("second@example.com", 2, Some("second"), 1),
    ];
    let time = Validator::new()
        .validate(&UniqueAccountSlice { users: &users })
        .unwrap_err()
        .into_fields()
        .unwrap();
    assert_eq!(param_list(&time[0], "fields"), vec!["created_at"]);
}

#[test]
fn unique_field_supports_bool_signed_integer_and_float_keys() {
    let error = Validator::new()
        .validate(&UniqueScalarFields {
            bools: vec![
                UniqueScalarItem {
                    enabled: true,
                    balance: 1,
                    ratio: 1.0,
                },
                UniqueScalarItem {
                    enabled: true,
                    balance: 2,
                    ratio: 2.0,
                },
            ],
            ints: vec![
                UniqueScalarItem {
                    enabled: true,
                    balance: -1,
                    ratio: 1.0,
                },
                UniqueScalarItem {
                    enabled: false,
                    balance: -1,
                    ratio: 2.0,
                },
            ],
            floats: vec![
                UniqueScalarItem {
                    enabled: true,
                    balance: 1,
                    ratio: -0.0,
                },
                UniqueScalarItem {
                    enabled: false,
                    balance: 2,
                    ratio: 0.0,
                },
            ],
        })
        .unwrap_err();
    let fields = error.into_fields().unwrap();

    assert_eq!(fields.len(), 3);
    assert_eq!(param_list(&fields[0], "fields"), vec!["enabled"]);
    assert_eq!(param_list(&fields[1], "fields"), vec!["balance"]);
    assert_eq!(param_list(&fields[2], "fields"), vec!["ratio"]);
}

#[test]
fn direct_unique_field_requires_field_context() {
    for expression in ["unique=email", "unique=tenant_id profile.email"] {
        let error = Validator::new()
            .value(
                &vec![serde_json::json!({ "email": "team@example.com" })],
                expression,
            )
            .unwrap_err();

        assert!(matches!(
            error,
            Error::MissingFieldContext { name } if name == "unique"
        ));
    }
}

#[test]
fn unique_field_rejects_unsupported_key_kind_as_configuration_error() {
    let error = Validator::new()
        .validate(&UniqueUnsupportedField {
            items: vec![UniqueUnsupportedItem {
                value: UnsupportedUniqueKey,
            }],
        })
        .unwrap_err();

    assert!(matches!(error, Error::InvalidRuleExpression { .. }));
}
