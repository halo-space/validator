use crate::Kind;

use super::{Context, Locale};

pub(super) fn locale() -> Locale {
    Locale::new("en")
        .rule("required", "{field} is required")
        .rule("isdefault", "{field} must be the default value")
        .rule("eq", "{field} must be equal to {value}")
        .rule("ne", "{field} must not be equal to {value}")
        .rule("eq_ignore_case", "{field} must be equal to {value}")
        .rule("ne_ignore_case", "{field} must not be equal to {value}")
        .rule("email", "{field} must be a valid email address")
        .rule("url", "{field} must be a valid URL")
        .rule("uri", "{field} must be a valid URI")
        .rule("http", "{field} must be a valid HTTP or HTTPS URL")
        .rule("https", "{field} must be a valid HTTPS URL")
        .rule("ip", "{field} must be a valid IP address")
        .rule("ipv4", "{field} must be a valid IPv4 address")
        .rule("ipv6", "{field} must be a valid IPv6 address")
        .rule("cidr", "{field} must be a valid CIDR block")
        .rule("cidrv4", "{field} must be a valid IPv4 CIDR block")
        .rule("cidrv6", "{field} must be a valid IPv6 CIDR block")
        .rule("hostname", "{field} must be a valid hostname")
        .rule("hostname_port", "{field} must be a valid hostname and port")
        .rule(
            "hostname_rfc1123",
            "{field} must be a valid RFC1123 hostname",
        )
        .rule(
            "fqdn",
            "{field} must be a valid fully qualified domain name",
        )
        .rule("port", "{field} must be a valid port")
        .rule("uuid", "{field} must be a valid lowercase UUID")
        .rule("uuid3", "{field} must be a valid lowercase UUID v3")
        .rule("uuid4", "{field} must be a valid lowercase UUID v4")
        .rule("uuid5", "{field} must be a valid lowercase UUID v5")
        .rule("uuid_rfc4122", "{field} must be a valid RFC4122 UUID")
        .rule("uuid3_rfc4122", "{field} must be a valid RFC4122 UUID v3")
        .rule("uuid4_rfc4122", "{field} must be a valid RFC4122 UUID v4")
        .rule("uuid5_rfc4122", "{field} must be a valid RFC4122 UUID v5")
        .rule("ulid", "{field} must be a valid ULID")
        .rule("tcp4", "{field} must be a valid IPv4 TCP address")
        .rule("tcp6", "{field} must be a valid IPv6 TCP address")
        .rule("tcp", "{field} must be a valid TCP address")
        .rule("udp4", "{field} must be a valid IPv4 UDP address")
        .rule("udp6", "{field} must be a valid IPv6 UDP address")
        .rule("udp", "{field} must be a valid UDP address")
        .rule("json", "{field} must be valid JSON")
        .rule("datetime", "{field} must be a valid datetime")
        .rule("regex", "{field} format is invalid")
        .rule("e164", "{field} must be a valid E.164 phone number")
        .rule("base32", "{field} must be a valid Base32 string")
        .rule("base64", "{field} must be a valid Base64 string")
        .rule("base64url", "{field} must be a valid Base64URL string")
        .rule(
            "base64rawurl",
            "{field} must be a valid unpadded Base64URL string",
        )
        .rule("hexadecimal", "{field} must be a valid hexadecimal string")
        .rule("url_encoded", "{field} must be a valid URL-encoded string")
        .rule("html", "{field} must contain a valid HTML tag")
        .rule("html_encoded", "{field} must contain a valid HTML entity")
        .rule("jwt", "{field} must be a valid JWT string")
        .rule("mac", "{field} must be a valid MAC address")
        .rule("semver", "{field} must be a valid semantic version")
        .rule("origin", "{field} must be a valid HTTP(S) origin")
        .rule("datauri", "{field} must be a valid Data URI")
        .rule("latitude", "{field} must be a valid latitude")
        .rule("longitude", "{field} must be a valid longitude")
        .rule("ssn", "{field} must be a valid SSN")
        .rule("md4", "{field} must be a valid MD4 hash")
        .rule("md5", "{field} must be a valid MD5 hash")
        .rule("sha256", "{field} must be a valid SHA256 hash")
        .rule("sha384", "{field} must be a valid SHA384 hash")
        .rule("sha512", "{field} must be a valid SHA512 hash")
        .rule("ripemd128", "{field} must be a valid RIPEMD128 hash")
        .rule("ripemd160", "{field} must be a valid RIPEMD160 hash")
        .rule("tiger128", "{field} must be a valid TIGER128 hash")
        .rule("tiger160", "{field} must be a valid TIGER160 hash")
        .rule("tiger192", "{field} must be a valid TIGER192 hash")
        .rule("eth_addr", "{field} must be a valid Ethereum address")
        .rule("mongodb", "{field} must be a valid MongoDB ObjectID")
        .rule(
            "mongodb_connection_string",
            "{field} must be a valid MongoDB connection string",
        )
        .rule(
            "dns_rfc1035_label",
            "{field} must be a valid RFC1035 DNS label",
        )
        .rule("cve", "{field} must be a valid CVE identifier")
        .rule("cron", "{field} must be a valid cron expression")
        .rule("ein", "{field} must be a valid EIN")
        .rule(
            "bic_iso_9362_2014",
            "{field} must be a valid BIC/SWIFT code",
        )
        .rule("bic", "{field} must be a valid BIC/SWIFT code")
        .rule("isbn", "{field} must be a valid ISBN")
        .rule("isbn10", "{field} must be a valid ISBN-10")
        .rule("isbn13", "{field} must be a valid ISBN-13")
        .rule("issn", "{field} must be a valid ISSN")
        .rule("credit_card", "{field} must be a valid credit card number")
        .rule("luhn_checksum", "{field} must pass the Luhn checksum")
        .rule("oneof", "{field} must be one of: {values}")
        .rule("oneofci", "{field} must be one of: {values}")
        .rule("noneof", "{field} must not be one of: {values}")
        .rule("noneofci", "{field} must not be one of: {values}")
        .rule("unique", "{field} must contain unique values")
        .rule("contains", "{field} must contain {value}")
        .rule("containsany", "{field} must contain any of: {value}")
        .rule("containsrune", "{field} must contain rune {value}")
        .rule("excludes", "{field} must not contain {value}")
        .rule("excludesall", "{field} must not contain any of: {value}")
        .rule("excludesrune", "{field} must not contain rune {value}")
        .rule("startswith", "{field} must start with {value}")
        .rule("endswith", "{field} must end with {value}")
        .rule("startsnotwith", "{field} must not start with {value}")
        .rule("endsnotwith", "{field} must not end with {value}")
        .rule("ascii", "{field} must contain only ASCII characters")
        .rule(
            "printascii",
            "{field} must contain only printable ASCII characters",
        )
        .rule("multibyte", "{field} must contain a multibyte character")
        .rule("alpha", "{field} must contain only letters")
        .rule("alphaspace", "{field} must contain only letters and spaces")
        .rule("alphaunicode", "{field} must contain only Unicode letters")
        .rule("alphanum", "{field} must contain only letters and numbers")
        .rule(
            "alphanumspace",
            "{field} must contain only letters, numbers, and spaces",
        )
        .rule(
            "alphanumunicode",
            "{field} must contain only Unicode letters and numbers",
        )
        .rule("numeric", "{field} must be numeric")
        .rule("number", "{field} must be a number")
        .rule("lowercase", "{field} must be lowercase")
        .rule("uppercase", "{field} must be uppercase")
        .rule("boolean", "{field} must be a boolean")
        .rule("hexcolor", "{field} must be a hex color")
        .rule("rgb", "{field} must be an RGB color")
        .rule("rgba", "{field} must be an RGBA color")
        .rule("hsl", "{field} must be an HSL color")
        .rule("hsla", "{field} must be an HSLA color")
        .rule("cmyk", "{field} must be a CMYK color")
        .rule("iscolor", "{field} must be a valid color")
        .rule("type", "{field} has invalid type, expected {expected}")
        .rule("eq_field", "{field} must be equal to {compare}")
        .rule("ne_field", "{field} must not be equal to {compare}")
        .rule("gt_field", "{field} must be greater than {compare}")
        .rule(
            "gte_field",
            "{field} must be greater than or equal to {compare}",
        )
        .rule("lt_field", "{field} must be less than {compare}")
        .rule(
            "lte_field",
            "{field} must be less than or equal to {compare}",
        )
        .rule("fieldcontains", "{field} must contain {compare}")
        .rule("fieldexcludes", "{field} must not contain {compare}")
        .rule("required_if", "{field} is required")
        .rule("required_unless", "{field} is required")
        .rule("skip_unless", "{field} is required")
        .rule("required_with", "{field} is required")
        .rule("required_with_all", "{field} is required")
        .rule("required_without", "{field} is required")
        .rule("required_without_all", "{field} is required")
        .rule("excluded_if", "{field} must be empty")
        .rule("excluded_unless", "{field} must be empty")
        .rule("excluded_with", "{field} must be empty")
        .rule("excluded_with_all", "{field} must be empty")
        .rule("excluded_without", "{field} must be empty")
        .rule("excluded_without_all", "{field} must be empty")
        .rule_fn("length", en_length)
        .rule_fn("min", |ctx| en_size(ctx, "must be at least"))
        .rule_fn("max", |ctx| en_size(ctx, "must be at most"))
        .rule_fn("range", en_range)
        .rule_fn("gt", |ctx| en_compare(ctx, "must be greater than"))
        .rule_fn("gte", |ctx| {
            en_compare(ctx, "must be greater than or equal to")
        })
        .rule_fn("lt", |ctx| en_compare(ctx, "must be less than"))
        .rule_fn("lte", |ctx| {
            en_compare(ctx, "must be less than or equal to")
        })
}

fn en_length(ctx: &Context<'_>) -> String {
    if let Some(exact) = ctx.param("exact") {
        return en_size_text(ctx, "must be exactly", exact);
    }

    match (ctx.param("min"), ctx.param("max")) {
        (Some(min), Some(max)) => match ctx.kind() {
            Kind::String => format!("{} length must be between {min} and {max}", ctx.field()),
            Kind::Vec | Kind::Array | Kind::Slice | Kind::Map => {
                format!("{} size must be between {min} and {max}", ctx.field())
            }
            _ => format!("{} must be between {min} and {max}", ctx.field()),
        },
        (Some(min), None) => en_size_text(ctx, "must be at least", min),
        (None, Some(max)) => en_size_text(ctx, "must be at most", max),
        (None, None) => default_context_text(ctx),
    }
}

fn en_size(ctx: &Context<'_>, label: &str) -> String {
    let value = ctx
        .param("min")
        .or_else(|| ctx.param("max"))
        .unwrap_or_default();

    en_size_text(ctx, label, value)
}

fn en_size_text(ctx: &Context<'_>, label: &str, value: &str) -> String {
    match ctx.kind() {
        Kind::String => format!("{} length {label} {value}", ctx.field()),
        Kind::Vec | Kind::Array | Kind::Slice | Kind::Map => {
            format!("{} size {label} {value}", ctx.field())
        }
        _ => format!("{} {label} {value}", ctx.field()),
    }
}

fn default_context_text(ctx: &Context<'_>) -> String {
    format!("{} failed {}", ctx.namespace().as_str(), ctx.rule())
}

fn en_range(ctx: &Context<'_>) -> String {
    let min = ctx.param("min").unwrap_or_default();
    let max = ctx.param("max").unwrap_or_default();
    match ctx.kind() {
        Kind::String => format!("{} length must be between {min} and {max}", ctx.field()),
        Kind::Vec | Kind::Array | Kind::Slice | Kind::Map => {
            format!("{} size must be between {min} and {max}", ctx.field())
        }
        _ => format!("{} must be between {min} and {max}", ctx.field()),
    }
}

fn en_compare(ctx: &Context<'_>, label: &str) -> String {
    let value = ctx.param("value").unwrap_or_default();
    match ctx.kind() {
        Kind::String => format!("{} length {label} {value}", ctx.field()),
        Kind::Vec | Kind::Array | Kind::Slice | Kind::Map => {
            format!("{} size {label} {value}", ctx.field())
        }
        _ => format!("{} {label} {value}", ctx.field()),
    }
}
