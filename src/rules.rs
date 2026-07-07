mod alias;
mod choice;
mod compare;
mod format;
mod network;
mod required;
mod size;
mod string;

use crate::core::{Error, Rules};
pub(crate) use alias::load as load_aliases;
use choice::{NoneOf, OneOf};
use compare::{Eq, Gt, Gte, Lt, Lte, Ne};
use format::{Cmyk, Email, HexColor, Hsl, Hsla, RegexRule, Rgb, Rgba};
use network::{HttpUrl, HttpsUrl, Ip, Ipv4, Ipv6, UrlRule, Uuid};
use required::Required;
use size::{Length, Max, Min, Range};
use string::{
    Alpha, Alphanum, Ascii, Boolean, Contains, ContainsAny, EndsWith, Lowercase, Number, Numeric,
    StartsWith, Uppercase,
};

pub(crate) fn load_rules(rules: &mut Rules) -> Result<(), Error> {
    rules.insert("required", Required)?;
    rules.insert("length", Length)?;
    rules.insert("min", Min)?;
    rules.insert("max", Max)?;
    rules.insert("eq", Eq)?;
    rules.insert("ne", Ne)?;
    rules.insert("gt", Gt)?;
    rules.insert("gte", Gte)?;
    rules.insert("lt", Lt)?;
    rules.insert("lte", Lte)?;
    rules.insert("range", Range)?;
    rules.insert("email", Email)?;
    rules.insert("url", UrlRule)?;
    rules.insert("http_url", HttpUrl)?;
    rules.insert("https_url", HttpsUrl)?;
    rules.insert("ip", Ip)?;
    rules.insert("ipv4", Ipv4)?;
    rules.insert("ipv6", Ipv6)?;
    rules.insert("uuid", Uuid)?;
    rules.insert("regex", RegexRule)?;
    rules.insert("hexcolor", HexColor)?;
    rules.insert("rgb", Rgb)?;
    rules.insert("rgba", Rgba)?;
    rules.insert("hsl", Hsl)?;
    rules.insert("hsla", Hsla)?;
    rules.insert("cmyk", Cmyk)?;
    rules.insert("oneof", OneOf)?;
    rules.insert("noneof", NoneOf)?;
    rules.insert("contains", Contains)?;
    rules.insert("containsany", ContainsAny)?;
    rules.insert("startswith", StartsWith)?;
    rules.insert("endswith", EndsWith)?;
    rules.insert("ascii", Ascii)?;
    rules.insert("alpha", Alpha)?;
    rules.insert("alphanum", Alphanum)?;
    rules.insert("numeric", Numeric)?;
    rules.insert("number", Number)?;
    rules.insert("lowercase", Lowercase)?;
    rules.insert("uppercase", Uppercase)?;
    rules.insert("boolean", Boolean)?;
    Ok(())
}
