
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
fn url_uri_and_hostname_rules_match_go_boundaries() {
    let validator = Validator::new();

    for value in [
        "https://example.com/posts/1",
        "http://foobar.中文网/",
        "http://www.foo_bar.com/",
        "http://www.-foobar.com/",
        "mailto:someone@example.com",
        "irc://#channel@network",
        "file://path/to/file.txt",
        "file:///c:/Windows/file.txt",
        "file:////remotehost/path/file.txt",
    ] {
        validator.value(&value, "url").unwrap();
    }
    for value in ["/abs/test/dir", "1://example.com", "file:", "file:/"] {
        assert!(validator.value(&value, "url").is_err());
    }

    for value in [
        "https://example.com/path#fragment",
        "http://foobar.中文网/",
        "http://www.foo_bar.com/",
        "mailto:someone@example.com",
        "irc://#channel@network",
        "/abs/test/dir",
    ] {
        validator.value(&value, "uri").unwrap();
    }
    for value in ["foobar.com", "./rel/test/dir", ""] {
        assert!(validator.value(&value, "uri").is_err());
    }

    assert!(validator.value(&"1.foo.com", "hostname").is_err());
    validator.value(&"abc1234", "hostname").unwrap();
    validator.value(&"1.foo.com", "hostname_rfc1123").unwrap();
    validator.value(&"api.example.com.", "fqdn").unwrap();
    validator.value(&"test-site.test-site", "fqdn").unwrap();
    assert!(validator.value(&"api.example.123", "fqdn").is_err());
    assert!(validator.value(&"example", "fqdn").is_err());
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
fn lowercase_uuid4_and_uuid5_require_rfc_variant() {
    let validator = Validator::new();

    for (value, rule) in [
        ("550e8400-e29b-41d4-0716-446655440000", "uuid4"),
        ("987fbc97-4bed-5078-0f07-9141ba07c9f3", "uuid5"),
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
fn cidr_rules_preserve_address_prefix_and_network_semantics() {
    let validator = Validator::new();

    validator.value(&"192.168.0.1/24", "cidr").unwrap();
    validator.value(&"2001:db8::1/32", "cidr,cidrv6").unwrap();
    let fields = validator
        .value(&"192.168.0.1/24", "cidrv4")
        .unwrap_err()
        .into_fields()
        .unwrap();
    assert_eq!(fields[0].rule(), "cidrv4");
}

#[test]
fn canonical_ip_and_socket_rules_keep_distinct_semantics() {
    let validator = Validator::new();

    validator.value(&"127.0.0.1", "ipv4").unwrap();
    validator.value(&"::1", "ipv6").unwrap();
    validator.value(&"::1", "ip").unwrap();
    validator.value(&"127.0.0.1:80", "tcp4").unwrap();
    validator.value(&"[::1]:80", "tcp6").unwrap();
    validator.value(&"127.0.0.1:80", "tcp").unwrap();
    validator.value(&"[::1]:80", "udp").unwrap();
    validator.value(&"127.0.0.1:80", "udp4").unwrap();
    validator.value(&"[::1]:80", "udp6").unwrap();

    for (value, rule) in [
        ("127.0.0.1:80", "ipv4"),
        ("[::1]:80", "tcp4"),
        ("127.0.0.1:80", "tcp6"),
        (":80", "udp"),
        ("localhost:80", "tcp"),
    ] {
        let fields = validator
            .value(&value, rule)
            .unwrap_err()
            .into_fields()
            .unwrap();

        assert_eq!(fields[0].rule(), rule);
    }

    for rule in [
        "ip_addr",
        "ip4_addr",
        "ip6_addr",
        "http_url",
        "https_url",
        "tcp_addr",
        "tcp4_addr",
        "tcp6_addr",
        "udp_addr",
        "udp4_addr",
        "udp6_addr",
    ] {
        let error = validator.value(&"127.0.0.1", rule).unwrap_err();
        assert!(matches!(error, Error::UnknownRule { name } if name == rule));
    }
}
