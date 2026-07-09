mod alias;
mod choice;
mod collection;
mod compare;
mod format;
mod network;
mod required;
mod size;
mod string;

use crate::core::{Error, Rules};
pub(crate) use alias::load as load_aliases;
use choice::{NoneOf, NoneOfCi, OneOf, OneOfCi};
use collection::Unique;
pub(crate) use collection::values_are_unique;
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
    Cidr, Cidrv4, Cidrv6, Fqdn, Hostname, HostnameRfc1123, HttpUrl, HttpsUrl, Ip, Ip4Address,
    Ip6Address, IpAddress, Ipv4, Ipv6, Port, Tcp4Address, Tcp6Address, TcpAddress, Udp4Address,
    Udp6Address, UdpAddress, Ulid, Uri, Url, Uuid, Uuid3, Uuid3Rfc4122, Uuid4, Uuid4Rfc4122, Uuid5,
    Uuid5Rfc4122, UuidRfc4122,
};
use required::Required;
use size::{Length, Max, Min, Range};
use string::{
    Alpha, AlphaSpace, AlphaUnicode, Alphanum, AlphanumSpace, AlphanumUnicode, Ascii, Boolean,
    Contains, ContainsAny, ContainsRune, EndsNotWith, EndsWith, Excludes, ExcludesAll,
    ExcludesRune, Lowercase, Multibyte, Number, Numeric, PrintAscii, StartsNotWith, StartsWith,
    Uppercase,
};

pub(crate) fn load(rules: &mut Rules) -> Result<(), Error> {
    rules.insert("required", Required)?;
    rules.insert("length", Length)?;
    rules.insert("min", Min)?;
    rules.insert("max", Max)?;
    rules.insert("eq", Eq)?;
    rules.insert("ne", Ne)?;
    rules.insert("eq_ignore_case", EqIgnoreCase)?;
    rules.insert("ne_ignore_case", NeIgnoreCase)?;
    rules.insert("gt", Gt)?;
    rules.insert("gte", Gte)?;
    rules.insert("lt", Lt)?;
    rules.insert("lte", Lte)?;
    rules.insert("range", Range)?;
    rules.insert("email", Email)?;
    rules.insert("url", Url)?;
    rules.insert("uri", Uri)?;
    rules.insert("http_url", HttpUrl)?;
    rules.insert("https_url", HttpsUrl)?;
    rules.insert("ip", Ip)?;
    rules.insert("ipv4", Ipv4)?;
    rules.insert("ipv6", Ipv6)?;
    rules.insert("ip_addr", IpAddress)?;
    rules.insert("ip4_addr", Ip4Address)?;
    rules.insert("ip6_addr", Ip6Address)?;
    rules.insert("cidr", Cidr)?;
    rules.insert("cidrv4", Cidrv4)?;
    rules.insert("cidrv6", Cidrv6)?;
    rules.insert("hostname", Hostname)?;
    rules.insert("hostname_rfc1123", HostnameRfc1123)?;
    rules.insert("fqdn", Fqdn)?;
    rules.insert("port", Port)?;
    rules.insert("uuid", Uuid)?;
    rules.insert("uuid3", Uuid3)?;
    rules.insert("uuid4", Uuid4)?;
    rules.insert("uuid5", Uuid5)?;
    rules.insert("uuid_rfc4122", UuidRfc4122)?;
    rules.insert("uuid3_rfc4122", Uuid3Rfc4122)?;
    rules.insert("uuid4_rfc4122", Uuid4Rfc4122)?;
    rules.insert("uuid5_rfc4122", Uuid5Rfc4122)?;
    rules.insert("ulid", Ulid)?;
    rules.insert("tcp4_addr", Tcp4Address)?;
    rules.insert("tcp6_addr", Tcp6Address)?;
    rules.insert("tcp_addr", TcpAddress)?;
    rules.insert("udp4_addr", Udp4Address)?;
    rules.insert("udp6_addr", Udp6Address)?;
    rules.insert("udp_addr", UdpAddress)?;
    rules.insert("json", Json)?;
    rules.insert("datetime", DateTime)?;
    rules.insert("regex", Regex::default())?;
    rules.insert("e164", E164)?;
    rules.insert("base32", Base32)?;
    rules.insert("base64", Base64)?;
    rules.insert("base64url", Base64Url)?;
    rules.insert("base64rawurl", Base64RawUrl)?;
    rules.insert("hexadecimal", Hexadecimal)?;
    rules.insert("url_encoded", UrlEncoded)?;
    rules.insert("html", Html)?;
    rules.insert("html_encoded", HtmlEncoded)?;
    rules.insert("jwt", Jwt)?;
    rules.insert("mac", Mac)?;
    rules.insert("semver", Semver)?;
    rules.insert("origin", Origin)?;
    rules.insert("datauri", DataUri)?;
    rules.insert("latitude", Latitude)?;
    rules.insert("longitude", Longitude)?;
    rules.insert("ssn", Ssn)?;
    rules.insert("md4", Md4)?;
    rules.insert("md5", Md5)?;
    rules.insert("sha256", Sha256)?;
    rules.insert("sha384", Sha384)?;
    rules.insert("sha512", Sha512)?;
    rules.insert("ripemd128", Ripemd128)?;
    rules.insert("ripemd160", Ripemd160)?;
    rules.insert("tiger128", Tiger128)?;
    rules.insert("tiger160", Tiger160)?;
    rules.insert("tiger192", Tiger192)?;
    rules.insert("eth_addr", EthAddr)?;
    rules.insert("mongodb", Mongodb)?;
    rules.insert("mongodb_connection_string", MongodbConnectionString)?;
    rules.insert("dns_rfc1035_label", DnsRfc1035Label)?;
    rules.insert("cve", Cve)?;
    rules.insert("cron", Cron)?;
    rules.insert("ein", Ein)?;
    rules.insert("bic_iso_9362_2014", BicIso93622014)?;
    rules.insert("bic", Bic)?;
    rules.insert("isbn", Isbn)?;
    rules.insert("isbn10", Isbn10)?;
    rules.insert("isbn13", Isbn13)?;
    rules.insert("issn", Issn)?;
    rules.insert("credit_card", CreditCard)?;
    rules.insert("luhn_checksum", LuhnChecksum)?;
    rules.insert("hexcolor", HexColor)?;
    rules.insert("rgb", Rgb)?;
    rules.insert("rgba", Rgba)?;
    rules.insert("hsl", Hsl)?;
    rules.insert("hsla", Hsla)?;
    rules.insert("cmyk", Cmyk)?;
    rules.insert("oneof", OneOf)?;
    rules.insert("oneofci", OneOfCi)?;
    rules.insert("noneof", NoneOf)?;
    rules.insert("noneofci", NoneOfCi)?;
    rules.insert("unique", Unique)?;
    rules.insert("contains", Contains)?;
    rules.insert("containsany", ContainsAny)?;
    rules.insert("containsrune", ContainsRune)?;
    rules.insert("excludes", Excludes)?;
    rules.insert("excludesall", ExcludesAll)?;
    rules.insert("excludesrune", ExcludesRune)?;
    rules.insert("startswith", StartsWith)?;
    rules.insert("endswith", EndsWith)?;
    rules.insert("startsnotwith", StartsNotWith)?;
    rules.insert("endsnotwith", EndsNotWith)?;
    rules.insert("ascii", Ascii)?;
    rules.insert("printascii", PrintAscii)?;
    rules.insert("multibyte", Multibyte)?;
    rules.insert("alpha", Alpha)?;
    rules.insert("alphaspace", AlphaSpace)?;
    rules.insert("alphaunicode", AlphaUnicode)?;
    rules.insert("alphanum", Alphanum)?;
    rules.insert("alphanumspace", AlphanumSpace)?;
    rules.insert("alphanumunicode", AlphanumUnicode)?;
    rules.insert("numeric", Numeric)?;
    rules.insert("number", Number)?;
    rules.insert("lowercase", Lowercase)?;
    rules.insert("uppercase", Uppercase)?;
    rules.insert("boolean", Boolean)?;
    Ok(())
}
