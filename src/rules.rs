mod alias;
mod choice;
mod collection;
mod compare;
mod field;
mod format;
mod network;
mod required;
mod size;
mod string;

use crate::core::{Error, Registry};
pub(crate) use alias::load as load_aliases;
use choice::{NoneOf, NoneOfCi, OneOf, OneOfCi};
use collection::Unique;
use compare::{Eq, EqIgnoreCase, Gt, Gte, Lt, Lte, Ne, NeIgnoreCase};
use format::{
    Base32, Base64, Base64RawUrl, Base64Url, Bic, BicIso93622014, Cmyk, CreditCard, Cron, Cve,
    DataUri, DateTime, DnsRfc1035Label, E164, Ein, Email, EthAddr, HexColor, Hexadecimal, Hsl,
    Hsla, Html, HtmlEncoded, Isbn, Isbn10, Isbn13, Issn, Json, Jwt, Latitude, Longitude,
    LuhnChecksum, Mac, Md4, Md5, Mongodb, MongodbConnectionString, Origin, Regex, Rgb, Rgba,
    Ripemd128, Ripemd160, Semver, Sha256, Sha384, Sha512, Ssn, Tiger128, Tiger160, Tiger192,
    UrlEncoded,
};
use network::{
    Cidr, Cidrv4, Cidrv6, Fqdn, Hostname, HostnamePort, HostnameRfc1123, HttpUrl, HttpsUrl, Ip,
    Ipv4, Ipv6, Port, Tcp4Address, Tcp6Address, TcpAddress, Udp4Address, Udp6Address, UdpAddress,
    Ulid, Uri, Url, Uuid, Uuid3, Uuid3Rfc4122, Uuid4, Uuid4Rfc4122, Uuid5, Uuid5Rfc4122,
    UuidRfc4122,
};
use required::{IsDefault, Required};
use size::{Length, Max, Min, Range};
use string::{
    Alpha, AlphaSpace, AlphaUnicode, Alphanum, AlphanumSpace, AlphanumUnicode, Ascii, Boolean,
    Contains, ContainsAny, ContainsRune, EndsNotWith, EndsWith, Excludes, ExcludesAll,
    ExcludesRune, Lowercase, Multibyte, Number, Numeric, PrintAscii, StartsNotWith, StartsWith,
    Uppercase,
};

pub(crate) fn load(rules: &mut Registry) -> Result<(), Error> {
    rules.rule("required", Required)?;
    rules.rule("isdefault", IsDefault)?;
    rules.rule("length", Length)?;
    rules.rule("min", Min)?;
    rules.rule("max", Max)?;
    rules.rule("eq", Eq)?;
    rules.rule("ne", Ne)?;
    rules.rule("eq_ignore_case", EqIgnoreCase)?;
    rules.rule("ne_ignore_case", NeIgnoreCase)?;
    rules.rule("gt", Gt)?;
    rules.rule("gte", Gte)?;
    rules.rule("lt", Lt)?;
    rules.rule("lte", Lte)?;
    rules.rule("range", Range)?;
    rules.rule("email", Email)?;
    rules.rule("url", Url)?;
    rules.rule("uri", Uri)?;
    rules.rule("http_url", HttpUrl)?;
    rules.rule("https_url", HttpsUrl)?;
    rules.rule("ip", Ip)?;
    rules.rule("ipv4", Ipv4)?;
    rules.rule("ipv6", Ipv6)?;
    rules.rule("cidr", Cidr)?;
    rules.rule("cidrv4", Cidrv4)?;
    rules.rule("cidrv6", Cidrv6)?;
    rules.rule("hostname", Hostname)?;
    rules.rule("hostname_port", HostnamePort)?;
    rules.rule("hostname_rfc1123", HostnameRfc1123)?;
    rules.rule("fqdn", Fqdn)?;
    rules.rule("port", Port)?;
    rules.rule("uuid", Uuid)?;
    rules.rule("uuid3", Uuid3)?;
    rules.rule("uuid4", Uuid4)?;
    rules.rule("uuid5", Uuid5)?;
    rules.rule("uuid_rfc4122", UuidRfc4122)?;
    rules.rule("uuid3_rfc4122", Uuid3Rfc4122)?;
    rules.rule("uuid4_rfc4122", Uuid4Rfc4122)?;
    rules.rule("uuid5_rfc4122", Uuid5Rfc4122)?;
    rules.rule("ulid", Ulid)?;
    rules.rule("tcp4_addr", Tcp4Address)?;
    rules.rule("tcp6_addr", Tcp6Address)?;
    rules.rule("tcp_addr", TcpAddress)?;
    rules.rule("udp4_addr", Udp4Address)?;
    rules.rule("udp6_addr", Udp6Address)?;
    rules.rule("udp_addr", UdpAddress)?;
    rules.rule("json", Json)?;
    rules.rule("datetime", DateTime)?;
    rules.rule("regex", Regex::default())?;
    rules.rule("e164", E164)?;
    rules.rule("base32", Base32)?;
    rules.rule("base64", Base64)?;
    rules.rule("base64url", Base64Url)?;
    rules.rule("base64rawurl", Base64RawUrl)?;
    rules.rule("hexadecimal", Hexadecimal)?;
    rules.rule("url_encoded", UrlEncoded)?;
    rules.rule("html", Html)?;
    rules.rule("html_encoded", HtmlEncoded)?;
    rules.rule("jwt", Jwt)?;
    rules.rule("mac", Mac)?;
    rules.rule("semver", Semver)?;
    rules.rule("origin", Origin)?;
    rules.rule("datauri", DataUri)?;
    rules.rule("latitude", Latitude)?;
    rules.rule("longitude", Longitude)?;
    rules.rule("ssn", Ssn)?;
    rules.rule("md4", Md4)?;
    rules.rule("md5", Md5)?;
    rules.rule("sha256", Sha256)?;
    rules.rule("sha384", Sha384)?;
    rules.rule("sha512", Sha512)?;
    rules.rule("ripemd128", Ripemd128)?;
    rules.rule("ripemd160", Ripemd160)?;
    rules.rule("tiger128", Tiger128)?;
    rules.rule("tiger160", Tiger160)?;
    rules.rule("tiger192", Tiger192)?;
    rules.rule("eth_addr", EthAddr)?;
    rules.rule("mongodb", Mongodb)?;
    rules.rule("mongodb_connection_string", MongodbConnectionString)?;
    rules.rule("dns_rfc1035_label", DnsRfc1035Label)?;
    rules.rule("cve", Cve)?;
    rules.rule("cron", Cron)?;
    rules.rule("ein", Ein)?;
    rules.rule("bic_iso_9362_2014", BicIso93622014)?;
    rules.rule("bic", Bic)?;
    rules.rule("isbn", Isbn)?;
    rules.rule("isbn10", Isbn10)?;
    rules.rule("isbn13", Isbn13)?;
    rules.rule("issn", Issn)?;
    rules.rule("credit_card", CreditCard)?;
    rules.rule("luhn_checksum", LuhnChecksum)?;
    rules.rule("hexcolor", HexColor)?;
    rules.rule("rgb", Rgb)?;
    rules.rule("rgba", Rgba)?;
    rules.rule("hsl", Hsl)?;
    rules.rule("hsla", Hsla)?;
    rules.rule("cmyk", Cmyk)?;
    rules.rule("oneof", OneOf)?;
    rules.rule("oneofci", OneOfCi)?;
    rules.rule("noneof", NoneOf)?;
    rules.rule("noneofci", NoneOfCi)?;
    rules.rule("unique", Unique)?;
    rules.rule("contains", Contains)?;
    rules.rule("containsany", ContainsAny)?;
    rules.rule("containsrune", ContainsRune)?;
    rules.rule("excludes", Excludes)?;
    rules.rule("excludesall", ExcludesAll)?;
    rules.rule("excludesrune", ExcludesRune)?;
    rules.rule("startswith", StartsWith)?;
    rules.rule("endswith", EndsWith)?;
    rules.rule("startsnotwith", StartsNotWith)?;
    rules.rule("endsnotwith", EndsNotWith)?;
    rules.rule("ascii", Ascii)?;
    rules.rule("printascii", PrintAscii)?;
    rules.rule("multibyte", Multibyte)?;
    rules.rule("alpha", Alpha)?;
    rules.rule("alphaspace", AlphaSpace)?;
    rules.rule("alphaunicode", AlphaUnicode)?;
    rules.rule("alphanum", Alphanum)?;
    rules.rule("alphanumspace", AlphanumSpace)?;
    rules.rule("alphanumunicode", AlphanumUnicode)?;
    rules.rule("numeric", Numeric)?;
    rules.rule("number", Number)?;
    rules.rule("lowercase", Lowercase)?;
    rules.rule("uppercase", Uppercase)?;
    rules.rule("boolean", Boolean)?;
    field::load(rules)?;
    Ok(())
}
