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

    #[validate(hostname_port)]
    hostname_port: String,

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
        hostname_port: "api.example.com:443".to_owned(),
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
        hostname_port: "[::1]:443".to_owned(),
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
            "hostname_port",
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
fn hostname_port_matches_go_split_host_port_semantics() {
    let validator = Validator::new();

    validator.value(&":8080", "hostname_port").unwrap();
    validator
        .value(&"api.example.com:443", "hostname_port")
        .unwrap();
    validator.value(&"127.0.0.1:443", "hostname_port").unwrap();

    for value in [
        "api.example.com",
        "api.example.com:0",
        "api.example.com:65536",
        "api_example.com:443",
        "[::1]:443",
    ] {
        let fields = validator
            .value(&value, "hostname_port")
            .unwrap_err()
            .into_fields()
            .unwrap();

        assert_eq!(fields[0].rule(), "hostname_port");
    }
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

#[test]
fn rfc4122_uuid_rules_accept_uppercase_and_version_boundaries() {
    let validator = Validator::new();

    validator
        .value(&"a987Fbc9-4bed-3078-cf07-9141ba07c9f3", "uuid_rfc4122")
        .unwrap();
    validator
        .value(&"a987fbc9-4bed-3078-cf07-9141ba07c9F3", "uuid3_rfc4122")
        .unwrap();
    validator
        .value(&"57b73598-8764-4ad0-a76A-679bb6640eb1", "uuid4_rfc4122")
        .unwrap();
    validator
        .value(&"987Fbc97-4bed-5078-9f07-9141ba07c9f3", "uuid5_rfc4122")
        .unwrap();

    for (value, rule) in [
        ("a987fbc9-4bed-5078-af07-9141ba07c9F3", "uuid4_rfc4122"),
        ("9c858901-8a57-4791-81Fe-4c455b099bc9", "uuid5_rfc4122"),
        ("aaaaaaaa-1111-1111-aaaG-111111111111", "uuid_rfc4122"),
    ] {
        let fields = validator
            .value(&value, rule)
            .unwrap_err()
            .into_fields()
            .unwrap();

        assert_eq!(fields[0].rule(), rule);
    }
}

#[test]
fn address_compatibility_rules_match_ip_literal_socket_semantics() {
    let validator = Validator::new();

    validator.value(&"127.0.0.1", "ip4_addr").unwrap();
    validator.value(&"::1", "ip6_addr").unwrap();
    validator.value(&"::1", "ip_addr").unwrap();
    validator.value(&"127.0.0.1:80", "tcp4_addr").unwrap();
    validator.value(&"[::1]:80", "tcp6_addr").unwrap();
    validator.value(&"127.0.0.1:80", "tcp_addr").unwrap();
    validator.value(&"[::1]:80", "udp_addr").unwrap();
    validator.value(&"127.0.0.1:80", "udp4_addr").unwrap();
    validator.value(&"[::1]:80", "udp6_addr").unwrap();

    for (value, rule) in [
        ("127.0.0.1:80", "ip4_addr"),
        ("[::1]:80", "tcp4_addr"),
        ("127.0.0.1:80", "tcp6_addr"),
        (":80", "udp_addr"),
        ("localhost:80", "tcp_addr"),
    ] {
        let fields = validator
            .value(&value, rule)
            .unwrap_err()
            .into_fields()
            .unwrap();

        assert_eq!(fields[0].rule(), rule);
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

    #[validate(e164)]
    phone: String,

    #[validate(base32)]
    base32: String,

    #[validate(base64)]
    base64: String,

    #[validate(base64url)]
    base64url: String,

    #[validate(base64rawurl)]
    base64rawurl: String,

    #[validate(hexadecimal)]
    hexadecimal: String,

    #[validate(url_encoded)]
    url_encoded: String,

    #[validate(html)]
    html: String,

    #[validate(html_encoded)]
    html_encoded: String,

    #[validate(jwt)]
    jwt: String,

    #[validate(mac)]
    mac: String,

    #[validate(semver)]
    semver: String,
}

#[test]
fn json_datetime_rules_pass() {
    let value = DocumentFormats {
        json: r#"{"ok":true}"#.to_owned(),
        datetime: "2026-07-08T12:30:00Z".to_owned(),
        phone: "+14155552671".to_owned(),
        base32: "MZXW6===".to_owned(),
        base64: "aGVsbG8=".to_owned(),
        base64url: "aGVsbG8=".to_owned(),
        base64rawurl: "aGVsbG8".to_owned(),
        hexadecimal: "0xdeadBEEF".to_owned(),
        url_encoded: "hello%20rust".to_owned(),
        html: "<section>".to_owned(),
        html_encoded: "&lt;".to_owned(),
        jwt: "eyJhbGciOiJOT05FIn0.e30.".to_owned(),
        mac: "01:23:45:67:89:ab".to_owned(),
        semver: "1.2.3-alpha.1+build.5".to_owned(),
    };

    Validator::new().validate(&value).unwrap();
}

#[test]
fn json_datetime_rules_fail() {
    let value = DocumentFormats {
        json: "{not-json}".to_owned(),
        datetime: "2026-13-08T12:30:00Z".to_owned(),
        phone: "+0123".to_owned(),
        base32: "mzxw6===".to_owned(),
        base64: "aGVsbG8".to_owned(),
        base64url: "aGVsbG8+".to_owned(),
        base64rawurl: "aGVsbG8=".to_owned(),
        hexadecimal: "0xnot-hex".to_owned(),
        url_encoded: "%test%".to_owned(),
        html: "<123nonsense>".to_owned(),
        html_encoded: "&x00".to_owned(),
        jwt: "eyJhbGciOiJOT05FIn0.e30.\n".to_owned(),
        mac: "01:23:45:67:89".to_owned(),
        semver: "1.2.3-0123".to_owned(),
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
            "json",
            "datetime",
            "e164",
            "base32",
            "base64",
            "base64url",
            "base64rawurl",
            "hexadecimal",
            "url_encoded",
            "html",
            "html_encoded",
            "jwt",
            "mac",
            "semver",
        ]
    );
}

#[test]
fn mac_accepts_common_notations() {
    let validator = Validator::new();

    validator.value(&"01:23:45:67:89:ab", "mac").unwrap();
    validator.value(&"01-23-45-67-89-ab", "mac").unwrap();
    validator.value(&"0123.4567.89ab", "mac").unwrap();
}

#[test]
fn extended_format_rules_pass() {
    let validator = Validator::new();

    for (value, rule) in [
        ("https://example.com:443", "origin"),
        ("data:text/plain;base64,aGVsbG8=", "datauri"),
        ("90.0", "latitude"),
        ("-180", "longitude"),
        ("123-45-6789", "ssn"),
        ("0x0123456789abcdef0123456789ABCDEF01234567", "eth_addr"),
        ("507f1f77bcf86cd799439011", "mongodb"),
        ("mongodb://localhost:27017", "mongodb_connection_string"),
        ("example", "dns_rfc1035_label"),
        ("CVE-2024-12345", "cve"),
        ("@daily", "cron"),
        ("12-3456789", "ein"),
        ("SBICKEN1345", "bic_iso_9362_2014"),
        ("DEUTDEFF500", "bic"),
        ("978-4-87311-368-5", "isbn"),
        ("3 401 01319 X", "isbn10"),
        ("9784873113685", "isbn13"),
        ("1050-124X", "issn"),
        ("4624 7482 3324 9780", "credit_card"),
        ("586824160825533338", "luhn_checksum"),
    ] {
        validator.value(&value, rule).unwrap();
    }

    validator.value(&90_i32, "latitude").unwrap();
    validator.value(&180_u32, "longitude").unwrap();
    validator.value(&10000000116_i64, "luhn_checksum").unwrap();
    validator
        .value(&586824160825533338_u64, "luhn_checksum")
        .unwrap();

    for (value, rule) in [
        ("a".repeat(32), "md4"),
        ("a".repeat(32), "md5"),
        ("a".repeat(64), "sha256"),
        ("a".repeat(96), "sha384"),
        ("a".repeat(128), "sha512"),
        ("a".repeat(32), "ripemd128"),
        ("a".repeat(40), "ripemd160"),
        ("a".repeat(32), "tiger128"),
        ("a".repeat(40), "tiger160"),
        ("a".repeat(48), "tiger192"),
    ] {
        validator.value(&value, rule).unwrap();
    }
}

#[test]
fn extended_format_rules_fail() {
    let validator = Validator::new();

    for (value, rule) in [
        ("https://example.com/path", "origin"),
        ("data:text/plain,hello", "datauri"),
        ("91", "latitude"),
        ("181", "longitude"),
        ("123456789", "ssn"),
        ("0x0123456789abcdef0123456789ABCDEF0123456", "eth_addr"),
        ("507f1f77bcf86cd79943901z", "mongodb"),
        (
            "mongodb+srv://localhost:27017?",
            "mongodb_connection_string",
        ),
        ("Example", "dns_rfc1035_label"),
        ("CVE-2024-0000", "cve"),
        ("invalid-cron", "cron"),
        ("123456789", "ein"),
        ("SBICKENXX", "bic_iso_9362_2014"),
        ("deUTDEFF", "bic"),
        ("foo", "isbn"),
        ("3423214121", "isbn10"),
        ("978 3 8362 2119 0", "isbn13"),
        ("2051-999X", "issn"),
        ("4624 7482 3324 978A", "credit_card"),
        ("586824160825533328", "luhn_checksum"),
    ] {
        let fields = validator
            .value(&value, rule)
            .unwrap_err()
            .into_fields()
            .unwrap();

        assert_eq!(fields[0].rule(), rule);
    }

    for (value, rule) in [
        ("A".repeat(32), "md5"),
        ("a".repeat(63), "sha256"),
        ("g".repeat(40), "ripemd160"),
    ] {
        let fields = validator
            .value(&value, rule)
            .unwrap_err()
            .into_fields()
            .unwrap();

        assert_eq!(fields[0].rule(), rule);
    }
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

#[test]
fn case_insensitive_choice_rules_work_for_strings() {
    let validator = Validator::new();

    validator.value(&"ReD", "oneofci(red,green)").unwrap();
    validator
        .value(&"BLUE", "oneofci('red green',blue)")
        .unwrap();
    validator.value(&"yellow", "noneofci(red,green)").unwrap();

    let fields = validator
        .value(&"RED", "noneofci(red,green)")
        .unwrap_err()
        .into_fields()
        .unwrap();

    assert_eq!(fields[0].rule(), "noneofci");
    assert_eq!(fields[0].params().get("values"), Some("red,green"));
}

#[derive(Debug, Validate)]
struct ChoiceCaseRules {
    #[validate(oneofci("draft", "published"))]
    state: String,

    #[validate(noneofci("root", "admin"))]
    username: String,
}

#[test]
fn derive_case_insensitive_choice_rules_work() {
    let value = ChoiceCaseRules {
        state: "PUBLISHED".to_owned(),
        username: "ADMIN".to_owned(),
    };

    let fields = Validator::new()
        .validate(&value)
        .unwrap_err()
        .into_fields()
        .unwrap();

    assert_eq!(fields.len(), 1);
    assert_eq!(fields[0].rule(), "noneofci");
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

    #[validate(containsrune(value = "好"))]
    greeting: String,

    #[validate(excludes(value = "admin"))]
    display_name: String,

    #[validate(excludesall(value = "!@#"))]
    slug: String,

    #[validate(excludesrune(value = "☻"))]
    mood: String,

    #[validate(startswith(value = "usr_"))]
    username: String,

    #[validate(endswith(value = ".rs"))]
    path: String,

    #[validate(startsnotwith(value = "tmp"))]
    storage_key: String,

    #[validate(endsnotwith(value = ".bak"))]
    filename: String,
}

#[test]
fn string_helpers_pass() {
    let value = TextHelpers {
        body: "hello rust".to_owned(),
        password: "hello!".to_owned(),
        greeting: "你好".to_owned(),
        display_name: "alice".to_owned(),
        slug: "hello-rust".to_owned(),
        mood: "happy".to_owned(),
        username: "usr_alice".to_owned(),
        path: "main.rs".to_owned(),
        storage_key: "cache/file".to_owned(),
        filename: "report.txt".to_owned(),
    };

    Validator::new().validate(&value).unwrap();
}

#[test]
fn string_helpers_fail() {
    let value = TextHelpers {
        body: "hello go".to_owned(),
        password: "hello".to_owned(),
        greeting: "hello".to_owned(),
        display_name: "root-admin".to_owned(),
        slug: "hello@rust".to_owned(),
        mood: "a☻b".to_owned(),
        username: "admin".to_owned(),
        path: "main.go".to_owned(),
        storage_key: "tmp/file".to_owned(),
        filename: "report.bak".to_owned(),
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
            "contains",
            "containsany",
            "containsrune",
            "excludes",
            "excludesall",
            "excludesrune",
            "startswith",
            "endswith",
            "startsnotwith",
            "endsnotwith",
        ]
    );
    assert_eq!(fields[1].params().get("value"), Some("!@#?"));
}

#[derive(Debug, Validate)]
struct CharacterClasses {
    #[validate(ascii)]
    ascii: String,

    #[validate(printascii)]
    printascii: String,

    #[validate(multibyte)]
    multibyte: String,

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
        printascii: "Hello, Rust!".to_owned(),
        multibyte: "你好".to_owned(),
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
        printascii: "hello\n".to_owned(),
        multibyte: "hello".to_owned(),
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

    assert_eq!(fields.len(), 10);
    assert_eq!(fields[0].rule(), "ascii");
    assert_eq!(fields[1].rule(), "printascii");
    assert_eq!(fields[2].rule(), "multibyte");
    assert_eq!(fields[3].rule(), "alpha");
    assert_eq!(fields[4].rule(), "alphanum");
    assert_eq!(fields[5].rule(), "numeric");
    assert_eq!(fields[6].rule(), "number");
    assert_eq!(fields[7].rule(), "lowercase");
    assert_eq!(fields[8].rule(), "uppercase");
    assert_eq!(fields[9].rule(), "boolean");
}

#[derive(Debug, Validate)]
struct ExtendedCharacterClasses {
    #[validate(alphaspace)]
    alphaspace: String,

    #[validate(alphanumspace)]
    alphanumspace: String,

    #[validate(alphaunicode)]
    alphaunicode: String,

    #[validate(alphanumunicode)]
    alphanumunicode: String,

    #[validate(eq_ignore_case = "Rust")]
    exact: String,

    #[validate(ne_ignore_case = "admin")]
    username: String,
}

#[test]
fn extended_character_classes_pass() {
    let value = ExtendedCharacterClasses {
        alphaspace: "Hello Rust".to_owned(),
        alphanumspace: "Rust 2024".to_owned(),
        alphaunicode: "你好Rust".to_owned(),
        alphanumunicode: "你好Rust2024".to_owned(),
        exact: "rust".to_owned(),
        username: "alice".to_owned(),
    };

    Validator::new().validate(&value).unwrap();
}

#[test]
fn extended_character_classes_fail() {
    let value = ExtendedCharacterClasses {
        alphaspace: "Hello 2024".to_owned(),
        alphanumspace: "Rust-2024".to_owned(),
        alphaunicode: "你好2024".to_owned(),
        alphanumunicode: "你好_Rust".to_owned(),
        exact: "Go".to_owned(),
        username: "ADMIN".to_owned(),
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
            "alphaspace",
            "alphanumspace",
            "alphaunicode",
            "alphanumunicode",
            "eq_ignore_case",
            "ne_ignore_case",
        ]
    );
    assert_eq!(fields[4].params().get("value"), Some("Rust"));
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
