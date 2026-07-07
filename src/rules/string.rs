mod alpha;
mod alphanum;
mod ascii;
mod boolean;
mod contains;
mod containsany;
mod endswith;
mod lowercase;
mod number;
mod numeric;
mod startswith;
mod uppercase;

use std::borrow::Cow;

use crate::Field;

pub(super) use alpha::Alpha;
pub(super) use alphanum::Alphanum;
pub(super) use ascii::Ascii;
pub(super) use boolean::Boolean;
pub(super) use contains::Contains;
pub(super) use containsany::ContainsAny;
pub(super) use endswith::EndsWith;
pub(super) use lowercase::Lowercase;
pub(super) use number::Number;
pub(super) use numeric::Numeric;
pub(super) use startswith::StartsWith;
pub(super) use uppercase::Uppercase;

fn value_and_expected<'a>(
    field: &'a Field<'_>,
    expected_name: &str,
) -> Option<(Cow<'a, str>, &'a str)> {
    Some((field.value().string()?, field.params().get(expected_name)?))
}
