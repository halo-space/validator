use std::collections::{BTreeMap, HashMap};

use validator::prelude::*;

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
    assert_eq!(fields[0].params().get("min"), Some("3"));
    assert_eq!(fields[1].rule(), "max");
    assert_eq!(fields[1].params().get("max"), Some("2"));
    assert_eq!(fields[2].rule(), "range");
    assert_eq!(fields[2].params().get("min"), Some("10"));
    assert_eq!(fields[2].params().get("max"), Some("20"));
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
    assert_eq!(fields[0].params().get("value"), Some("130"));
    assert_eq!(fields[2].params().get("value"), Some("-10"));
    assert_eq!(fields[3].params().get("value"), Some("3.5"));
}

#[derive(Debug, Validate)]
struct Formats {
    #[validate(email)]
    email: String,

    #[validate(url)]
    source_url: String,

    #[validate(uri)]
    urn: String,

    #[validate(regex(pattern = "^[a-z0-9-]+$"))]
    slug: String,
}

#[test]
fn email_url_regex_pass() {
    let value = Formats {
        email: "team@example.com".to_owned(),
        source_url: "https://example.com/posts/1".to_owned(),
        urn: "urn:isbn:0451450523".to_owned(),
        slug: "hello-rust-2024".to_owned(),
    };

    Validator::new().validate(&value).unwrap();
}

#[test]
fn email_url_regex_fail() {
    let value = Formats {
        email: "not-email".to_owned(),
        source_url: "example.com/posts/1".to_owned(),
        urn: "not uri".to_owned(),
        slug: "Hello Rust".to_owned(),
    };

    let fields = Validator::new()
        .validate(&value)
        .unwrap_err()
        .into_fields()
        .unwrap();

    assert_eq!(fields.len(), 4);
    assert_eq!(fields[0].rule(), "email");
    assert_eq!(fields[1].rule(), "url");
    assert_eq!(fields[2].rule(), "uri");
    assert_eq!(fields[3].rule(), "regex");
    assert_eq!(fields[3].params().get("pattern"), Some("^[a-z0-9-]+$"));
}

#[test]
fn repeated_dynamic_regex_validation_preserves_errors() {
    let validator = Validator::new();
    let expression = r#"regex(pattern="^[a-z0-9-]+$")"#;

    validator.value(&"hello-rust", expression).unwrap();
    let first = validator
        .value(&"Hello Rust", expression)
        .unwrap_err()
        .into_fields()
        .unwrap();
    let second = validator
        .value(&"Hello Rust", expression)
        .unwrap_err()
        .into_fields()
        .unwrap();

    assert_eq!(first, second);
}

#[derive(Debug, Validate)]
struct NetworkFormats {
    #[validate(cidr)]
    cidr: String,

    #[validate(cidrv4)]
    cidrv4: String,

    #[validate(cidrv6)]
    cidrv6: String,

    #[validate(hostname)]
    hostname: String,

    #[validate(hostname_rfc1123)]
    hostname_rfc1123: String,

    #[validate(fqdn)]
    fqdn: String,

    #[validate(port)]
    port: String,

    #[validate(uuid3)]
    uuid3: String,

    #[validate(uuid4)]
    uuid4: String,

    #[validate(uuid5)]
    uuid5: String,

    #[validate(ulid)]
    ulid: String,
}

#[test]
fn expanded_network_rules_pass() {
    let value = NetworkFormats {
        cidr: "192.168.0.0/24".to_owned(),
        cidrv4: "10.0.0.0/8".to_owned(),
        cidrv6: "2001:db8::/32".to_owned(),
        hostname: "api".to_owned(),
        hostname_rfc1123: "1.foo.com".to_owned(),
        fqdn: "api.example.com".to_owned(),
        port: "443".to_owned(),
        uuid3: "a987fbc9-4bed-3078-cf07-9141ba07c9f3".to_owned(),
        uuid4: "550e8400-e29b-41d4-a716-446655440000".to_owned(),
        uuid5: "987fbc97-4bed-5078-af07-9141ba07c9f3".to_owned(),
        ulid: "01BX5ZZKBKACTAV9WEVGEMMVRZ".to_owned(),
    };

    Validator::new().validate(&value).unwrap();
}

#[test]
fn expanded_network_rules_fail() {
    let value = NetworkFormats {
        cidr: "192.168.0.0/33".to_owned(),
        cidrv4: "2001:db8::/32".to_owned(),
        cidrv6: "10.0.0.0/8".to_owned(),
        hostname: "-api".to_owned(),
        hostname_rfc1123: "foo_bar.example.com".to_owned(),
        fqdn: "api".to_owned(),
        port: "0".to_owned(),
        uuid3: "550e8400-e29b-41d4-a716-446655440000".to_owned(),
        uuid4: "a987fbc9-4bed-3078-cf07-9141ba07c9f3".to_owned(),
        uuid5: "550e8400-e29b-41d4-a716-446655440000".to_owned(),
        ulid: "O1BX5ZZKBKACTAV9WEVGEMMVRZ".to_owned(),
    };

    let fields = Validator::new()
        .validate(&value)
        .unwrap_err()
        .into_fields()
        .unwrap();
    let rules = fields.iter().map(|field| field.rule()).collect::<Vec<_>>();

    assert_eq!(
        rules,
        vec![
            "cidr",
            "cidrv4",
            "cidrv6",
            "hostname",
            "hostname_rfc1123",
            "fqdn",
            "port",
            "uuid3",
            "uuid4",
            "uuid5",
            "ulid",
        ]
    );
}

#[test]
fn hostname_rfc1123_accepts_digit_prefix_and_rejects_invalid_boundaries() {
    let validator = Validator::new();

    validator.value(&"1.foo.com", "hostname_rfc1123").unwrap();
    validator.value(&"192.168.0.1", "hostname_rfc1123").unwrap();

    for value in [
        "test_example",
        "example.",
        "example..com",
        "-example.com",
        "example-.com",
        "foo.bar:80",
        "this-is-a-deliberately-overlong-subdomain-used-for-boundary-test.example.com",
    ] {
        let fields = validator
            .value(&value, "hostname_rfc1123")
            .unwrap_err()
            .into_fields()
            .unwrap();

        assert_eq!(fields[0].rule(), "hostname_rfc1123");
    }
}

#[test]
fn ulid_rejects_ambiguous_characters_and_wrong_length() {
    let validator = Validator::new();

    for value in [
        "0IBX5ZZKBKACTAV9WEVGEMMVRZ",
        "01BX5ZZKBKACTAVLWEVGEMMVRZ",
        "O1BX5ZZKBKACTAV9WEVGEMMVRZ",
        "01BX5ZZKBKACTAV9WEVGEMMVRU",
        "01BX5ZZKBKACTAV9WEVGEMMVRZABC",
    ] {
        let fields = validator
            .value(&value, "ulid")
            .unwrap_err()
            .into_fields()
            .unwrap();

        assert_eq!(fields[0].rule(), "ulid");
    }
}

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
fn unique_rejects_unsupported_json_array_elements() {
    let fields = Validator::new()
        .value(&serde_json::json!([{ "id": 1 }, { "id": 2 }]), "unique")
        .unwrap_err()
        .into_fields()
        .unwrap();

    assert_eq!(fields[0].rule(), "unique");
}

#[derive(Debug, Validate)]
struct DocumentFormats {
    #[validate(json)]
    json: String,

    #[validate(datetime)]
    datetime: String,
}

#[test]
fn json_datetime_rules_pass() {
    let value = DocumentFormats {
        json: r#"{"ok":true}"#.to_owned(),
        datetime: "2026-07-08T12:30:00Z".to_owned(),
    };

    Validator::new().validate(&value).unwrap();
}

#[test]
fn json_datetime_rules_fail() {
    let value = DocumentFormats {
        json: "{not-json}".to_owned(),
        datetime: "2026-13-08T12:30:00Z".to_owned(),
    };

    let fields = Validator::new()
        .validate(&value)
        .unwrap_err()
        .into_fields()
        .unwrap();
    let rules = fields.iter().map(|field| field.rule()).collect::<Vec<_>>();

    assert_eq!(rules, vec!["json", "datetime"]);
}

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

#[derive(Debug, Validate)]
struct ColorFormats {
    #[validate(hexcolor)]
    hex: String,

    #[validate(rgb)]
    rgb: String,

    #[validate(rgba)]
    rgba: String,

    #[validate(hsl)]
    hsl: String,

    #[validate(hsla)]
    hsla: String,

    #[validate(cmyk)]
    cmyk: String,
}

#[test]
fn color_rules_pass() {
    let value = ColorFormats {
        hex: "#00ffaa".to_owned(),
        rgb: "rgb(255, 0, 120)".to_owned(),
        rgba: "rgba(255, 0, 120, 0.5)".to_owned(),
        hsl: "hsl(360, 100%, 50%)".to_owned(),
        hsla: "hsla(240, 100%, 50%, 1)".to_owned(),
        cmyk: "cmyk(0%, 10%, 20%, 100%)".to_owned(),
    };

    Validator::new().validate(&value).unwrap();
}

#[test]
fn color_rules_fail() {
    let value = ColorFormats {
        hex: "#000-".to_owned(),
        rgb: "rgb(256, 0, 0)".to_owned(),
        rgba: "rgba(0, 0, 0, 1.5)".to_owned(),
        hsl: "hsl(361, 100%, 50%)".to_owned(),
        hsla: "hsla(240, 100%, 50%, 2)".to_owned(),
        cmyk: "cmyk(0%, 10%, 20%, 101%)".to_owned(),
    };

    let fields = Validator::new()
        .validate(&value)
        .unwrap_err()
        .into_fields()
        .unwrap();
    let rules = fields.iter().map(|field| field.rule()).collect::<Vec<_>>();

    assert_eq!(
        rules,
        vec!["hexcolor", "rgb", "rgba", "hsl", "hsla", "cmyk"]
    );
}

#[derive(Debug, Validate)]
struct FavoriteColor {
    #[validate(alias = "iscolor")]
    color: String,
}

#[test]
fn default_iscolor_alias_accepts_any_color_format() {
    let hex = FavoriteColor {
        color: "#fff".to_owned(),
    };
    let rgb = FavoriteColor {
        color: "rgb(255, 255, 255)".to_owned(),
    };

    Validator::new().validate(&hex).unwrap();
    Validator::new().validate(&rgb).unwrap();
}

#[test]
fn default_iscolor_alias_reports_alias_failure() {
    let value = FavoriteColor {
        color: "#000-".to_owned(),
    };

    let fields = Validator::new()
        .validate(&value)
        .unwrap_err()
        .into_fields()
        .unwrap();

    assert_eq!(fields.len(), 1);
    assert_eq!(fields[0].rule(), "iscolor");
    assert_eq!(fields[0].reason(), "hexcolor|rgb|rgba|hsl|hsla|cmyk");
}

#[derive(Debug, Validate)]
struct PublishState {
    #[validate(oneof("draft", "published"))]
    status: String,
}

#[derive(Debug, Validate)]
struct Priority {
    #[validate(oneof(1, 2, 3))]
    level: u8,
}

#[test]
fn oneof_passes_for_allowed_value() {
    let value = PublishState {
        status: "draft".to_owned(),
    };

    Validator::new().validate(&value).unwrap();
}

#[test]
fn oneof_reports_values() {
    let value = PublishState {
        status: "archived".to_owned(),
    };

    let fields = Validator::new()
        .validate(&value)
        .unwrap_err()
        .into_fields()
        .unwrap();

    assert_eq!(fields.len(), 1);
    assert_eq!(fields[0].rule(), "oneof");
    assert_eq!(fields[0].params().get("values"), Some("draft,published"));
}

#[test]
fn oneof_dispatches_integer_candidates_by_field_type() {
    let value = Priority { level: 2 };

    Validator::new().validate(&value).unwrap();
}

#[derive(Debug, Validate)]
struct ReservedUsername {
    #[validate(noneof("root", "admin"))]
    username: String,
}

#[derive(Debug, Validate)]
struct ForbiddenLevel {
    #[validate(noneof(1, 2, 3))]
    level: i32,
}

#[test]
fn noneof_passes_for_unlisted_value() {
    let value = ReservedUsername {
        username: "alice".to_owned(),
    };

    Validator::new().validate(&value).unwrap();
}

#[test]
fn noneof_reports_values() {
    let value = ReservedUsername {
        username: "root".to_owned(),
    };

    let fields = Validator::new()
        .validate(&value)
        .unwrap_err()
        .into_fields()
        .unwrap();

    assert_eq!(fields.len(), 1);
    assert_eq!(fields[0].rule(), "noneof");
    assert_eq!(fields[0].params().get("values"), Some("root,admin"));
}

#[test]
fn noneof_dispatches_integer_candidates_by_field_type() {
    let value = ForbiddenLevel { level: 4 };

    Validator::new().validate(&value).unwrap();

    let value = ForbiddenLevel { level: 2 };
    let fields = Validator::new()
        .validate(&value)
        .unwrap_err()
        .into_fields()
        .unwrap();

    assert_eq!(fields.len(), 1);
    assert_eq!(fields[0].rule(), "noneof");
    assert_eq!(fields[0].params().get("values"), Some("1,2,3"));
}

#[derive(Debug, Validate)]
struct AliasState {
    #[validate(alias = "publish_state")]
    status: String,
}

#[test]
fn oneof_works_inside_alias_expression() -> Result<(), Box<dyn std::error::Error>> {
    let value = AliasState {
        status: "archived".to_owned(),
    };

    let fields = Validator::new()
        .alias("publish_state", r#"oneof("draft","published")"#)?
        .validate(&value)
        .unwrap_err()
        .into_fields()
        .unwrap();

    assert_eq!(fields.len(), 1);
    assert_eq!(fields[0].rule(), "publish_state");
    assert_eq!(fields[0].reason(), "oneof");
    assert_eq!(fields[0].params().get("values"), Some("draft,published"));

    Ok(())
}

#[derive(Debug, Validate)]
struct TextHelpers {
    #[validate(contains(value = "rust"))]
    body: String,

    #[validate(containsany(value = "!@#?"))]
    password: String,

    #[validate(startswith(value = "usr_"))]
    username: String,

    #[validate(endswith(value = ".rs"))]
    path: String,
}

#[test]
fn string_helpers_pass() {
    let value = TextHelpers {
        body: "hello rust".to_owned(),
        password: "hello!".to_owned(),
        username: "usr_alice".to_owned(),
        path: "main.rs".to_owned(),
    };

    Validator::new().validate(&value).unwrap();
}

#[test]
fn string_helpers_fail() {
    let value = TextHelpers {
        body: "hello go".to_owned(),
        password: "hello".to_owned(),
        username: "admin".to_owned(),
        path: "main.go".to_owned(),
    };

    let fields = Validator::new()
        .validate(&value)
        .unwrap_err()
        .into_fields()
        .unwrap();

    assert_eq!(fields.len(), 4);
    assert_eq!(fields[0].rule(), "contains");
    assert_eq!(fields[1].rule(), "containsany");
    assert_eq!(fields[1].params().get("value"), Some("!@#?"));
    assert_eq!(fields[2].rule(), "startswith");
    assert_eq!(fields[3].rule(), "endswith");
}

#[derive(Debug, Validate)]
struct CharacterClasses {
    #[validate(ascii)]
    ascii: String,

    #[validate(alpha)]
    alpha: String,

    #[validate(alphanum)]
    alphanum: String,

    #[validate(numeric)]
    numeric: String,

    #[validate(number)]
    number: String,

    #[validate(lowercase)]
    lowercase: String,

    #[validate(uppercase)]
    uppercase: String,

    #[validate(boolean)]
    boolean: String,
}

#[test]
fn character_classes_pass() {
    let value = CharacterClasses {
        ascii: "abc-123".to_owned(),
        alpha: "abcXYZ".to_owned(),
        alphanum: "abc123".to_owned(),
        numeric: "-12.5".to_owned(),
        number: "12345".to_owned(),
        lowercase: "rust".to_owned(),
        uppercase: "RUST".to_owned(),
        boolean: "true".to_owned(),
    };

    Validator::new().validate(&value).unwrap();
}

#[test]
fn character_classes_fail() {
    let value = CharacterClasses {
        ascii: "你好".to_owned(),
        alpha: "abc1".to_owned(),
        alphanum: "abc-123".to_owned(),
        numeric: "12e3".to_owned(),
        number: "12.3".to_owned(),
        lowercase: "Rust".to_owned(),
        uppercase: "Rust".to_owned(),
        boolean: "maybe".to_owned(),
    };

    let fields = Validator::new()
        .validate(&value)
        .unwrap_err()
        .into_fields()
        .unwrap();

    assert_eq!(fields.len(), 8);
    assert_eq!(fields[0].rule(), "ascii");
    assert_eq!(fields[1].rule(), "alpha");
    assert_eq!(fields[2].rule(), "alphanum");
    assert_eq!(fields[3].rule(), "numeric");
    assert_eq!(fields[4].rule(), "number");
    assert_eq!(fields[5].rule(), "lowercase");
    assert_eq!(fields[6].rule(), "uppercase");
    assert_eq!(fields[7].rule(), "boolean");
}

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
    assert_eq!(fields[0].params().get("value"), Some("published"));
    assert_eq!(fields[1].params().get("value"), Some("0"));
    assert_eq!(fields[2].params().get("value"), Some("true"));
    assert_eq!(fields[3].params().get("value"), Some("2"));
}

#[derive(Debug, Validate)]
struct NetworkRules {
    #[validate(http_url)]
    http_url: String,

    #[validate(https_url)]
    https_url: String,

    #[validate(ip)]
    ip: String,

    #[validate(ipv4)]
    ipv4: String,

    #[validate(ipv6)]
    ipv6: String,

    #[validate(uuid)]
    uuid: String,
}

#[test]
fn network_rules_pass() {
    let value = NetworkRules {
        http_url: "http://example.com".to_owned(),
        https_url: "https://example.com".to_owned(),
        ip: "::1".to_owned(),
        ipv4: "127.0.0.1".to_owned(),
        ipv6: "2001:db8::1".to_owned(),
        uuid: "a987fbc9-4bed-3078-cf07-9141ba07c9f3".to_owned(),
    };

    Validator::new().validate(&value).unwrap();
}

#[test]
fn network_rules_fail() {
    let value = NetworkRules {
        http_url: "ftp://example.com".to_owned(),
        https_url: "http://example.com".to_owned(),
        ip: "not-ip".to_owned(),
        ipv4: "::1".to_owned(),
        ipv6: "127.0.0.1".to_owned(),
        uuid: "A987FBC9-4BED-3078-CF07-9141BA07C9F3".to_owned(),
    };

    let fields = Validator::new()
        .validate(&value)
        .unwrap_err()
        .into_fields()
        .unwrap();
    let rules = fields.iter().map(|field| field.rule()).collect::<Vec<_>>();

    assert_eq!(
        rules,
        vec!["http_url", "https_url", "ip", "ipv4", "ipv6", "uuid"]
    );
}
