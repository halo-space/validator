
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
fn datetime_rejects_empty_fractional_seconds() {
    let fields = Validator::new()
        .value(&"2026-07-08T12:30:00.Z", "datetime")
        .unwrap_err()
        .into_fields()
        .unwrap();

    assert_eq!(fields[0].rule(), "datetime");
}

#[test]
fn mac_accepts_common_notations() {
    let validator = Validator::new();

    validator.value(&"01:23:45:67:89:ab", "mac").unwrap();
    validator.value(&"01-23-45-67-89-ab", "mac").unwrap();
    validator.value(&"0123.4567.89ab", "mac").unwrap();

    for value in [
        "02:00:5e:10:00:00:00:01",
        "02-00-5e-10-00-00-00-01",
        "0200.5e10.0000.0001",
        "00:00:00:00:fe:80:00:00:00:00:00:00:02:00:5e:10:00:00:00:01",
        "00-00-00-00-fe-80-00-00-00-00-00-00-02-00-5e-10-00-00-00-01",
        "0000.0000.fe80.0000.0000.0000.0200.5e10.0000.0001",
    ] {
        validator.value(&value, "mac").unwrap();
    }
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
