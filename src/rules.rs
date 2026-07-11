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
    Cidr, Cidrv4, Cidrv6, Fqdn, Hostname, HostnamePort, HostnameRfc1123, Http, Https, Ip, Ipv4,
    Ipv6, Port, Tcp, Tcp4, Tcp6, Udp, Udp4, Udp6, Ulid, Uri, Url, Uuid, Uuid3, Uuid3Rfc4122, Uuid4,
    Uuid4Rfc4122, Uuid5, Uuid5Rfc4122, UuidRfc4122,
};
use required::{IsDefault, Required};
use size::{Length, Max, Min, Range};
use string::{
    Alpha, AlphaSpace, AlphaUnicode, Alphanum, AlphanumSpace, AlphanumUnicode, Ascii, Boolean,
    Contains, ContainsAny, ContainsRune, EndsNotWith, EndsWith, Excludes, ExcludesAll,
    ExcludesRune, Lowercase, Multibyte, Number, Numeric, PrintAscii, StartsNotWith, StartsWith,
    Uppercase,
};

pub(crate) fn load(registry: &mut Registry) -> Result<(), Error> {
    registry.rule("required", Required)?;
    registry.rule("isdefault", IsDefault)?;
    registry.rule("length", Length)?;
    registry.rule("min", Min)?;
    registry.rule("max", Max)?;
    registry.rule("eq", Eq)?;
    registry.rule("ne", Ne)?;
    registry.rule("eq_ignore_case", EqIgnoreCase)?;
    registry.rule("ne_ignore_case", NeIgnoreCase)?;
    registry.rule("gt", Gt)?;
    registry.rule("gte", Gte)?;
    registry.rule("lt", Lt)?;
    registry.rule("lte", Lte)?;
    registry.rule("range", Range)?;
    registry.rule("email", Email)?;
    registry.rule("url", Url)?;
    registry.rule("uri", Uri)?;
    registry.rule("http", Http)?;
    registry.rule("https", Https)?;
    registry.rule("ip", Ip)?;
    registry.rule("ipv4", Ipv4)?;
    registry.rule("ipv6", Ipv6)?;
    registry.rule("cidr", Cidr)?;
    registry.rule("cidrv4", Cidrv4)?;
    registry.rule("cidrv6", Cidrv6)?;
    registry.rule("hostname", Hostname)?;
    registry.rule("hostname_port", HostnamePort)?;
    registry.rule("hostname_rfc1123", HostnameRfc1123)?;
    registry.rule("fqdn", Fqdn)?;
    registry.rule("port", Port)?;
    registry.rule("uuid", Uuid)?;
    registry.rule("uuid3", Uuid3)?;
    registry.rule("uuid4", Uuid4)?;
    registry.rule("uuid5", Uuid5)?;
    registry.rule("uuid_rfc4122", UuidRfc4122)?;
    registry.rule("uuid3_rfc4122", Uuid3Rfc4122)?;
    registry.rule("uuid4_rfc4122", Uuid4Rfc4122)?;
    registry.rule("uuid5_rfc4122", Uuid5Rfc4122)?;
    registry.rule("ulid", Ulid)?;
    registry.rule("tcp4", Tcp4)?;
    registry.rule("tcp6", Tcp6)?;
    registry.rule("tcp", Tcp)?;
    registry.rule("udp4", Udp4)?;
    registry.rule("udp6", Udp6)?;
    registry.rule("udp", Udp)?;
    registry.rule("json", Json)?;
    registry.rule("datetime", DateTime)?;
    registry.rule("regex", Regex::default())?;
    registry.rule("e164", E164)?;
    registry.rule("base32", Base32)?;
    registry.rule("base64", Base64)?;
    registry.rule("base64url", Base64Url)?;
    registry.rule("base64rawurl", Base64RawUrl)?;
    registry.rule("hexadecimal", Hexadecimal)?;
    registry.rule("url_encoded", UrlEncoded)?;
    registry.rule("html", Html)?;
    registry.rule("html_encoded", HtmlEncoded)?;
    registry.rule("jwt", Jwt)?;
    registry.rule("mac", Mac)?;
    registry.rule("semver", Semver)?;
    registry.rule("origin", Origin)?;
    registry.rule("datauri", DataUri)?;
    registry.rule("latitude", Latitude)?;
    registry.rule("longitude", Longitude)?;
    registry.rule("ssn", Ssn)?;
    registry.rule("md4", Md4)?;
    registry.rule("md5", Md5)?;
    registry.rule("sha256", Sha256)?;
    registry.rule("sha384", Sha384)?;
    registry.rule("sha512", Sha512)?;
    registry.rule("ripemd128", Ripemd128)?;
    registry.rule("ripemd160", Ripemd160)?;
    registry.rule("tiger128", Tiger128)?;
    registry.rule("tiger160", Tiger160)?;
    registry.rule("tiger192", Tiger192)?;
    registry.rule("eth_addr", EthAddr)?;
    registry.rule("mongodb", Mongodb)?;
    registry.rule("mongodb_connection_string", MongodbConnectionString)?;
    registry.rule("dns_rfc1035_label", DnsRfc1035Label)?;
    registry.rule("cve", Cve)?;
    registry.rule("cron", Cron)?;
    registry.rule("ein", Ein)?;
    registry.rule("bic_iso_9362_2014", BicIso93622014)?;
    registry.rule("bic", Bic)?;
    registry.rule("isbn", Isbn)?;
    registry.rule("isbn10", Isbn10)?;
    registry.rule("isbn13", Isbn13)?;
    registry.rule("issn", Issn)?;
    registry.rule("credit_card", CreditCard)?;
    registry.rule("luhn_checksum", LuhnChecksum)?;
    registry.rule("hexcolor", HexColor)?;
    registry.rule("rgb", Rgb)?;
    registry.rule("rgba", Rgba)?;
    registry.rule("hsl", Hsl)?;
    registry.rule("hsla", Hsla)?;
    registry.rule("cmyk", Cmyk)?;
    registry.rule("oneof", OneOf)?;
    registry.rule("oneofci", OneOfCi)?;
    registry.rule("noneof", NoneOf)?;
    registry.rule("noneofci", NoneOfCi)?;
    registry.rule("unique", Unique)?;
    registry.rule("contains", Contains)?;
    registry.rule("containsany", ContainsAny)?;
    registry.rule("containsrune", ContainsRune)?;
    registry.rule("excludes", Excludes)?;
    registry.rule("excludesall", ExcludesAll)?;
    registry.rule("excludesrune", ExcludesRune)?;
    registry.rule("startswith", StartsWith)?;
    registry.rule("endswith", EndsWith)?;
    registry.rule("startsnotwith", StartsNotWith)?;
    registry.rule("endsnotwith", EndsNotWith)?;
    registry.rule("ascii", Ascii)?;
    registry.rule("printascii", PrintAscii)?;
    registry.rule("multibyte", Multibyte)?;
    registry.rule("alpha", Alpha)?;
    registry.rule("alphaspace", AlphaSpace)?;
    registry.rule("alphaunicode", AlphaUnicode)?;
    registry.rule("alphanum", Alphanum)?;
    registry.rule("alphanumspace", AlphanumSpace)?;
    registry.rule("alphanumunicode", AlphanumUnicode)?;
    registry.rule("numeric", Numeric)?;
    registry.rule("number", Number)?;
    registry.rule("lowercase", Lowercase)?;
    registry.rule("uppercase", Uppercase)?;
    registry.rule("boolean", Boolean)?;
    field::load(registry)?;
    Ok(())
}
