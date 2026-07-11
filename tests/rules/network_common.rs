
#[derive(Debug, Validate)]
struct NetworkRules {
    #[validate(http)]
    http_url: String,

    #[validate(https)]
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

    assert_eq!(rules, vec!["http", "https", "ip", "ipv4", "ipv6", "uuid"]);
}
