use std::borrow::Cow;
use std::collections::BTreeMap;
use std::sync::Arc;

use serde::Deserialize;

use crate::{Error, FieldError, Kind, Namespace, Params};

pub type RenderFn = Arc<dyn for<'a> Fn(&Context<'a>) -> String + Send + Sync + 'static>;

#[derive(Clone)]
pub enum Template {
    Text(String),
    Fn(RenderFn),
}

#[derive(Clone, Default)]
pub struct I18n {
    locales: BTreeMap<String, Locale>,
    fallback: Option<String>,
}

impl I18n {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn zh_cn(self) -> Self {
        self.use_locale(zh_cn_locale())
    }

    pub fn en(self) -> Self {
        self.use_locale(en_locale())
    }

    pub fn use_locale(mut self, locale: Locale) -> Self {
        self.locales
            .entry(locale.name.clone())
            .and_modify(|current| current.merge(locale.clone()))
            .or_insert(locale);
        self
    }

    pub fn fallback(mut self, locale: impl Into<String>) -> Self {
        self.fallback = Some(locale.into());
        self
    }

    pub fn locale(&self, locale: impl AsRef<str>) -> Translator<'_> {
        let selected = self.locales.get(locale.as_ref()).or_else(|| {
            self.fallback
                .as_deref()
                .and_then(|name| self.locales.get(name))
        });

        Translator {
            locale: selected.map(Cow::Borrowed),
        }
    }
}

#[derive(Clone, Default)]
pub struct Locale {
    name: String,
    rules: BTreeMap<String, Template>,
    fields: BTreeMap<String, String>,
}

impl Locale {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            ..Self::default()
        }
    }

    pub fn from_yaml(yaml: impl AsRef<str>) -> Result<Self, Error> {
        let resource =
            serde_yaml::from_str::<LocaleResource>(yaml.as_ref()).map_err(invalid_locale_error)?;
        Self::from_resource(resource)
    }

    pub fn from_json(json: impl AsRef<str>) -> Result<Self, Error> {
        let resource =
            serde_json::from_str::<LocaleResource>(json.as_ref()).map_err(invalid_locale_error)?;
        Self::from_resource(resource)
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn rule(mut self, rule: impl Into<String>, template: impl Into<String>) -> Self {
        self.rules
            .insert(rule.into(), Template::Text(template.into()));
        self
    }

    pub fn rule_fn<F>(mut self, rule: impl Into<String>, render: F) -> Self
    where
        F: for<'a> Fn(&Context<'a>) -> String + Send + Sync + 'static,
    {
        self.rules
            .insert(rule.into(), Template::Fn(Arc::new(render)));
        self
    }

    pub fn template(mut self, rule: impl Into<String>, template: Template) -> Self {
        self.rules.insert(rule.into(), template);
        self
    }

    pub fn field(mut self, field: impl Into<String>, label: impl Into<String>) -> Self {
        self.fields.insert(field.into(), label.into());
        self
    }

    fn merge(&mut self, other: Locale) {
        self.rules.extend(other.rules);
        self.fields.extend(other.fields);
    }

    fn from_resource(resource: LocaleResource) -> Result<Self, Error> {
        let name = resource
            .locale
            .or(resource.name)
            .ok_or_else(|| invalid_locale("locale name is required"))?;
        if name.trim().is_empty() {
            return Err(invalid_locale("locale name is required"));
        }

        let mut locale = Self::new(name);
        for (rule, template) in resource.rules.unwrap_or_default() {
            locale = locale.rule(rule, template);
        }
        for (field, label) in resource.fields.unwrap_or_default() {
            locale = locale.field(field, label);
        }

        Ok(locale)
    }

    fn template_for(&self, error: &FieldError) -> Option<&Template> {
        self.rules
            .get(error.rule())
            .or_else(|| self.rules.get(error.reason()))
    }

    fn field_label<'a>(&'a self, error: &'a FieldError) -> &'a str {
        self.fields
            .get(error.field())
            .map(String::as_str)
            .unwrap_or_else(|| error.field())
    }
}

#[derive(Deserialize)]
struct LocaleResource {
    locale: Option<String>,
    name: Option<String>,
    rules: Option<BTreeMap<String, String>>,
    fields: Option<BTreeMap<String, String>>,
}

pub struct Translator<'a> {
    locale: Option<Cow<'a, Locale>>,
}

impl Translator<'_> {
    pub fn render(&self, fields: &[FieldError]) -> Vec<Message> {
        fields.iter().map(|field| self.render_one(field)).collect()
    }

    fn render_one(&self, error: &FieldError) -> Message {
        let display_field = self
            .locale
            .as_deref()
            .map(|locale| locale.field_label(error))
            .unwrap_or_else(|| error.field());
        let context = Context {
            error,
            field: display_field,
        };
        let text = self
            .locale
            .as_deref()
            .and_then(|locale| locale.template_for(error))
            .map(|template| render_template(template, &context))
            .unwrap_or_else(|| default_text(error));

        Message {
            namespace: error.namespace().clone(),
            struct_namespace: error.struct_namespace().clone(),
            field: display_field.to_owned(),
            struct_field: error.struct_field().to_owned(),
            rule: error.rule().to_owned(),
            reason: error.reason().to_owned(),
            kind: error.kind(),
            params: error.params().clone(),
            text,
        }
    }
}

pub struct Context<'a> {
    error: &'a FieldError,
    field: &'a str,
}

impl<'a> Context<'a> {
    pub fn namespace(&self) -> &Namespace {
        self.error.namespace()
    }

    pub fn struct_namespace(&self) -> &Namespace {
        self.error.struct_namespace()
    }

    pub fn field(&self) -> &str {
        self.field
    }

    pub fn struct_field(&self) -> &str {
        self.error.struct_field()
    }

    pub fn rule(&self) -> &str {
        self.error.rule()
    }

    pub fn reason(&self) -> &str {
        self.error.reason()
    }

    pub fn kind(&self) -> Kind {
        self.error.kind()
    }

    pub fn params(&self) -> &Params {
        self.error.params()
    }

    pub fn param(&self, name: &str) -> Option<&str> {
        self.error.params().get(name)
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Message {
    pub namespace: Namespace,
    pub struct_namespace: Namespace,
    pub field: String,
    pub struct_field: String,
    pub rule: String,
    pub reason: String,
    pub kind: Kind,
    pub params: Params,
    pub text: String,
}

pub fn new() -> I18n {
    I18n::new()
}

pub fn zh_cn() -> Translator<'static> {
    Translator {
        locale: Some(Cow::Owned(zh_cn_locale())),
    }
}

pub fn en() -> Translator<'static> {
    Translator {
        locale: Some(Cow::Owned(en_locale())),
    }
}

fn render_template(template: &Template, context: &Context<'_>) -> String {
    match template {
        Template::Text(template) => render_text(template, context),
        Template::Fn(render) => render(context),
    }
}

fn render_text(template: &str, context: &Context<'_>) -> String {
    let mut text = template
        .replace("{namespace}", context.namespace().as_str())
        .replace("{struct_namespace}", context.struct_namespace().as_str())
        .replace("{field}", context.field())
        .replace("{struct_field}", context.struct_field())
        .replace("{rule}", context.rule())
        .replace("{reason}", context.reason())
        .replace("{kind}", kind_name(context.kind()));

    for (name, value) in context.params().iter() {
        text = text.replace(&format!("{{{name}}}"), value);
    }

    text
}

fn default_text(error: &FieldError) -> String {
    format!("{} failed {}", error.namespace().as_str(), error.rule())
}

fn invalid_locale_error(error: impl std::error::Error) -> Error {
    invalid_locale(error.to_string())
}

fn invalid_locale(reason: impl Into<String>) -> Error {
    Error::InvalidData {
        reason: format!("invalid locale resource: {}", reason.into()),
    }
}

fn kind_name(kind: Kind) -> &'static str {
    match kind {
        Kind::String => "string",
        Kind::Bool => "bool",
        Kind::Int(_) => "int",
        Kind::Uint(_) => "uint",
        Kind::Float(_) => "float",
        Kind::Vec => "vec",
        Kind::Array => "array",
        Kind::Slice => "slice",
        Kind::Map => "map",
        Kind::Option => "option",
        Kind::Time => "time",
        Kind::Other => "other",
    }
}

fn zh_cn_locale() -> Locale {
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
        .rule("http_url", "{field}必须是有效的HTTP或HTTPS URL")
        .rule("https_url", "{field}必须是有效的HTTPS URL")
        .rule("ip", "{field}必须是有效的IP地址")
        .rule("ipv4", "{field}必须是有效的IPv4地址")
        .rule("ipv6", "{field}必须是有效的IPv6地址")
        .rule("ip_addr", "{field}必须是有效的IP地址")
        .rule("ip4_addr", "{field}必须是有效的IPv4地址")
        .rule("ip6_addr", "{field}必须是有效的IPv6地址")
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
        .rule("tcp4_addr", "{field}必须是有效的IPv4 TCP地址")
        .rule("tcp6_addr", "{field}必须是有效的IPv6 TCP地址")
        .rule("tcp_addr", "{field}必须是有效的TCP地址")
        .rule("udp4_addr", "{field}必须是有效的IPv4 UDP地址")
        .rule("udp6_addr", "{field}必须是有效的IPv6 UDP地址")
        .rule("udp_addr", "{field}必须是有效的UDP地址")
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
        .rule("required_with", "{field}不能为空")
        .rule("required_without", "{field}不能为空")
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

fn en_locale() -> Locale {
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
        .rule("http_url", "{field} must be a valid HTTP or HTTPS URL")
        .rule("https_url", "{field} must be a valid HTTPS URL")
        .rule("ip", "{field} must be a valid IP address")
        .rule("ipv4", "{field} must be a valid IPv4 address")
        .rule("ipv6", "{field} must be a valid IPv6 address")
        .rule("ip_addr", "{field} must be a valid IP address")
        .rule("ip4_addr", "{field} must be a valid IPv4 address")
        .rule("ip6_addr", "{field} must be a valid IPv6 address")
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
        .rule("tcp4_addr", "{field} must be a valid IPv4 TCP address")
        .rule("tcp6_addr", "{field} must be a valid IPv6 TCP address")
        .rule("tcp_addr", "{field} must be a valid TCP address")
        .rule("udp4_addr", "{field} must be a valid IPv4 UDP address")
        .rule("udp6_addr", "{field} must be a valid IPv6 UDP address")
        .rule("udp_addr", "{field} must be a valid UDP address")
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
        .rule("required_with", "{field} is required")
        .rule("required_without", "{field} is required")
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

#[cfg(test)]
mod tests {
    use crate::core::{Aliases, Rules};

    use super::*;

    const INTERNAL_ERROR_RULES: &[&str] = &[
        "type",
        "eq_field",
        "ne_field",
        "gt_field",
        "gte_field",
        "lt_field",
        "lte_field",
        "fieldcontains",
        "fieldexcludes",
        "required_if",
        "required_unless",
        "required_with",
        "required_without",
    ];

    #[test]
    fn built_in_locales_cover_default_rules_aliases_and_internal_errors() {
        let mut rules = Rules::new();
        crate::rules::load(&mut rules).expect("default rules must load");
        let mut aliases = Aliases::new();
        crate::rules::load_aliases(&mut aliases).expect("default aliases must load");

        let names = rules
            .names()
            .chain(aliases.names())
            .chain(INTERNAL_ERROR_RULES.iter().copied())
            .collect::<Vec<_>>();

        assert_locale_covers("zh-CN", &zh_cn_locale(), &names);
        assert_locale_covers("en", &en_locale(), &names);
    }

    fn assert_locale_covers(locale_name: &str, locale: &Locale, names: &[&str]) {
        let missing = names
            .iter()
            .copied()
            .filter(|name| !locale.rules.contains_key(*name))
            .collect::<Vec<_>>();

        assert!(
            missing.is_empty(),
            "{locale_name} missing i18n templates: {missing:?}"
        );
    }
}
