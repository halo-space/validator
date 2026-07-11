use crate::Kind;

use super::{Context, Locale};

pub(super) fn locale() -> Locale {
    Locale::new("zh-CN")
        .rule("required", "{field}不能为空")
        .rule("isdefault", "{field}必须是默认值")
        .rule("eq", "{field}必须等于{value}")
        .rule("ne", "{field}不能等于{value}")
        .rule("eq_ignore_case", "{field}必须等于{value}")
        .rule("ne_ignore_case", "{field}不能等于{value}")
        .rule("email", "{field}格式不正确")
        .rule("url", "{field}必须是有效的URL")
        .rule("uri", "{field}必须是有效的URI")
        .rule("http", "{field}必须是有效的HTTP或HTTPS URL")
        .rule("https", "{field}必须是有效的HTTPS URL")
        .rule("ip", "{field}必须是有效的IP地址")
        .rule("ipv4", "{field}必须是有效的IPv4地址")
        .rule("ipv6", "{field}必须是有效的IPv6地址")
        .rule("cidr", "{field}必须是有效的CIDR")
        .rule("cidrv4", "{field}必须是有效的IPv4 CIDR")
        .rule("cidrv6", "{field}必须是有效的IPv6 CIDR")
        .rule("hostname", "{field}必须是有效的主机名")
        .rule("hostname_port", "{field}必须是有效的主机名和端口")
        .rule("hostname_rfc1123", "{field}必须是有效的RFC1123主机名")
        .rule("fqdn", "{field}必须是有效的完整域名")
        .rule("port", "{field}必须是有效端口")
        .rule("uuid", "{field}必须是有效的小写UUID")
        .rule("uuid3", "{field}必须是有效的小写UUID v3")
        .rule("uuid4", "{field}必须是有效的小写UUID v4")
        .rule("uuid5", "{field}必须是有效的小写UUID v5")
        .rule("uuid_rfc4122", "{field}必须是有效的RFC4122 UUID")
        .rule("uuid3_rfc4122", "{field}必须是有效的RFC4122 UUID v3")
        .rule("uuid4_rfc4122", "{field}必须是有效的RFC4122 UUID v4")
        .rule("uuid5_rfc4122", "{field}必须是有效的RFC4122 UUID v5")
        .rule("ulid", "{field}必须是有效的ULID")
        .rule("tcp4", "{field}必须是有效的IPv4 TCP地址")
        .rule("tcp6", "{field}必须是有效的IPv6 TCP地址")
        .rule("tcp", "{field}必须是有效的TCP地址")
        .rule("udp4", "{field}必须是有效的IPv4 UDP地址")
        .rule("udp6", "{field}必须是有效的IPv6 UDP地址")
        .rule("udp", "{field}必须是有效的UDP地址")
        .rule("json", "{field}必须是有效的JSON")
        .rule("datetime", "{field}必须是有效的日期时间")
        .rule("regex", "{field}格式不正确")
        .rule("e164", "{field}必须是有效的E.164手机号")
        .rule("base32", "{field}必须是有效的Base32字符串")
        .rule("base64", "{field}必须是有效的Base64字符串")
        .rule("base64url", "{field}必须是有效的Base64URL字符串")
        .rule("base64rawurl", "{field}必须是有效的无填充Base64URL字符串")
        .rule("hexadecimal", "{field}必须是有效的十六进制字符串")
        .rule("url_encoded", "{field}必须是有效的URL编码字符串")
        .rule("html", "{field}必须包含有效的HTML标签")
        .rule("html_encoded", "{field}必须包含有效的HTML实体编码")
        .rule("jwt", "{field}必须是有效的JWT字符串")
        .rule("mac", "{field}必须是有效的MAC地址")
        .rule("semver", "{field}必须是有效的语义化版本号")
        .rule("origin", "{field}必须是有效的HTTP(S)源")
        .rule("datauri", "{field}必须是有效的Data URI")
        .rule("latitude", "{field}必须是有效的纬度")
        .rule("longitude", "{field}必须是有效的经度")
        .rule("ssn", "{field}必须是有效的SSN")
        .rule("md4", "{field}必须是有效的MD4哈希")
        .rule("md5", "{field}必须是有效的MD5哈希")
        .rule("sha256", "{field}必须是有效的SHA256哈希")
        .rule("sha384", "{field}必须是有效的SHA384哈希")
        .rule("sha512", "{field}必须是有效的SHA512哈希")
        .rule("ripemd128", "{field}必须是有效的RIPEMD128哈希")
        .rule("ripemd160", "{field}必须是有效的RIPEMD160哈希")
        .rule("tiger128", "{field}必须是有效的TIGER128哈希")
        .rule("tiger160", "{field}必须是有效的TIGER160哈希")
        .rule("tiger192", "{field}必须是有效的TIGER192哈希")
        .rule("eth_addr", "{field}必须是有效的以太坊地址")
        .rule("mongodb", "{field}必须是有效的MongoDB ObjectID")
        .rule(
            "mongodb_connection_string",
            "{field}必须是有效的MongoDB连接字符串",
        )
        .rule("dns_rfc1035_label", "{field}必须是有效的RFC1035 DNS标签")
        .rule("cve", "{field}必须是有效的CVE编号")
        .rule("cron", "{field}必须是有效的cron表达式")
        .rule("ein", "{field}必须是有效的EIN")
        .rule("bic_iso_9362_2014", "{field}必须是有效的BIC/SWIFT代码")
        .rule("bic", "{field}必须是有效的BIC/SWIFT代码")
        .rule("isbn", "{field}必须是有效的ISBN")
        .rule("isbn10", "{field}必须是有效的ISBN-10")
        .rule("isbn13", "{field}必须是有效的ISBN-13")
        .rule("issn", "{field}必须是有效的ISSN")
        .rule("credit_card", "{field}必须是有效的信用卡号")
        .rule("luhn_checksum", "{field}必须通过Luhn校验")
        .rule("oneof", "{field}必须是以下值之一：{values}")
        .rule("oneofci", "{field}必须是以下值之一：{values}")
        .rule("noneof", "{field}不能是以下值之一：{values}")
        .rule("noneofci", "{field}不能是以下值之一：{values}")
        .rule("unique", "{field}不能包含重复值")
        .rule("contains", "{field}必须包含{value}")
        .rule("containsany", "{field}必须包含以下任一字符：{value}")
        .rule("containsrune", "{field}必须包含字符{value}")
        .rule("excludes", "{field}不能包含{value}")
        .rule("excludesall", "{field}不能包含以下任一字符：{value}")
        .rule("excludesrune", "{field}不能包含字符{value}")
        .rule("startswith", "{field}必须以{value}开头")
        .rule("endswith", "{field}必须以{value}结尾")
        .rule("startsnotwith", "{field}不能以{value}开头")
        .rule("endsnotwith", "{field}不能以{value}结尾")
        .rule("ascii", "{field}只能包含ASCII字符")
        .rule("printascii", "{field}只能包含可打印ASCII字符")
        .rule("multibyte", "{field}必须包含多字节字符")
        .rule("alpha", "{field}只能包含字母")
        .rule("alphaspace", "{field}只能包含字母和空格")
        .rule("alphaunicode", "{field}只能包含Unicode字母")
        .rule("alphanum", "{field}只能包含字母和数字")
        .rule("alphanumspace", "{field}只能包含字母、数字和空格")
        .rule("alphanumunicode", "{field}只能包含Unicode字母和数字")
        .rule("numeric", "{field}必须是数字")
        .rule("number", "{field}必须是数字")
        .rule("lowercase", "{field}必须是小写")
        .rule("uppercase", "{field}必须是大写")
        .rule("boolean", "{field}必须是布尔值")
        .rule("hexcolor", "{field}必须是十六进制颜色")
        .rule("rgb", "{field}必须是RGB颜色")
        .rule("rgba", "{field}必须是RGBA颜色")
        .rule("hsl", "{field}必须是HSL颜色")
        .rule("hsla", "{field}必须是HSLA颜色")
        .rule("cmyk", "{field}必须是CMYK颜色")
        .rule("iscolor", "{field}必须是有效颜色")
        .rule("type", "{field}类型不正确，期望{expected}")
        .rule("eq_field", "{field}必须等于{compare}")
        .rule("ne_field", "{field}不能等于{compare}")
        .rule("gt_field", "{field}必须大于{compare}")
        .rule("gte_field", "{field}必须大于或等于{compare}")
        .rule("lt_field", "{field}必须小于{compare}")
        .rule("lte_field", "{field}必须小于或等于{compare}")
        .rule("fieldcontains", "{field}必须包含{compare}")
        .rule("fieldexcludes", "{field}不能包含{compare}")
        .rule("required_if", "{field}不能为空")
        .rule("required_unless", "{field}不能为空")
        .rule("skip_unless", "{field}不能为空")
        .rule("required_with", "{field}不能为空")
        .rule("required_with_all", "{field}不能为空")
        .rule("required_without", "{field}不能为空")
        .rule("required_without_all", "{field}不能为空")
        .rule("excluded_if", "{field}必须为空")
        .rule("excluded_unless", "{field}必须为空")
        .rule("excluded_with", "{field}必须为空")
        .rule("excluded_with_all", "{field}必须为空")
        .rule("excluded_without", "{field}必须为空")
        .rule("excluded_without_all", "{field}必须为空")
        .rule_fn("length", zh_length)
        .rule_fn("min", |ctx| {
            zh_size(ctx, "长度不能小于", "数量不能小于", "不能小于")
        })
        .rule_fn("max", |ctx| {
            zh_size(ctx, "长度不能大于", "数量不能大于", "不能大于")
        })
        .rule_fn("range", zh_range)
        .rule_fn("gt", |ctx| zh_compare(ctx, "必须大于"))
        .rule_fn("gte", |ctx| zh_compare(ctx, "必须大于或等于"))
        .rule_fn("lt", |ctx| zh_compare(ctx, "必须小于"))
        .rule_fn("lte", |ctx| zh_compare(ctx, "必须小于或等于"))
}

fn zh_length(ctx: &Context<'_>) -> String {
    if let Some(exact) = ctx.param("exact") {
        return zh_size_text(ctx, "长度必须等于", "数量必须等于", "必须等于", exact);
    }

    match (ctx.param("min"), ctx.param("max")) {
        (Some(min), Some(max)) => match ctx.kind() {
            Kind::String => format!("{}长度必须在{min}到{max}之间", ctx.field()),
            Kind::Vec | Kind::Array | Kind::Slice | Kind::Map => {
                format!("{}数量必须在{min}到{max}之间", ctx.field())
            }
            _ => format!("{}必须在{min}到{max}之间", ctx.field()),
        },
        (Some(min), None) => zh_size_text(ctx, "长度不能小于", "数量不能小于", "不能小于", min),
        (None, Some(max)) => zh_size_text(ctx, "长度不能大于", "数量不能大于", "不能大于", max),
        (None, None) => default_context_text(ctx),
    }
}

fn zh_size(
    ctx: &Context<'_>,
    string_label: &str,
    collection_label: &str,
    value_label: &str,
) -> String {
    let value = ctx
        .param("min")
        .or_else(|| ctx.param("max"))
        .unwrap_or_default();

    zh_size_text(ctx, string_label, collection_label, value_label, value)
}

fn zh_size_text(
    ctx: &Context<'_>,
    string_label: &str,
    collection_label: &str,
    value_label: &str,
    value: &str,
) -> String {
    let label = match ctx.kind() {
        Kind::String => string_label,
        Kind::Vec | Kind::Array | Kind::Slice | Kind::Map => collection_label,
        _ => value_label,
    };

    format!("{}{}{value}", ctx.field(), label)
}

fn zh_range(ctx: &Context<'_>) -> String {
    let min = ctx.param("min").unwrap_or_default();
    let max = ctx.param("max").unwrap_or_default();
    match ctx.kind() {
        Kind::String => format!("{}长度必须在{min}到{max}之间", ctx.field()),
        Kind::Vec | Kind::Array | Kind::Slice | Kind::Map => {
            format!("{}数量必须在{min}到{max}之间", ctx.field())
        }
        _ => format!("{}必须在{min}到{max}之间", ctx.field()),
    }
}

fn zh_compare(ctx: &Context<'_>, label: &str) -> String {
    let value = ctx.param("value").unwrap_or_default();
    match ctx.kind() {
        Kind::String => format!("{}长度{label}{value}", ctx.field()),
        Kind::Vec | Kind::Array | Kind::Slice | Kind::Map => {
            format!("{}数量{label}{value}", ctx.field())
        }
        _ => format!("{}{label}{value}", ctx.field()),
    }
}

fn default_context_text(ctx: &Context<'_>) -> String {
    format!("{} failed {}", ctx.namespace().as_str(), ctx.rule())
}
