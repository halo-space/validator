mod alpha;
mod alpha_num_space;
mod alpha_num_unicode;
mod alpha_space;
mod alpha_unicode;
mod alphanum;
mod ascii;
mod boolean;
mod contains;
mod contains_any;
mod contains_rune;
mod ends_not_with;
mod ends_with;
mod excludes;
mod excludes_all;
mod excludes_rune;
mod lowercase;
mod multibyte;
mod number;
mod numeric;
mod print_ascii;
mod starts_not_with;
mod starts_with;
mod uppercase;

use std::borrow::Cow;

use crate::Field;

pub(super) use alpha::Alpha;
pub(super) use alpha_num_space::AlphanumSpace;
pub(super) use alpha_num_unicode::AlphanumUnicode;
pub(super) use alpha_space::AlphaSpace;
pub(super) use alpha_unicode::AlphaUnicode;
pub(super) use alphanum::Alphanum;
pub(super) use ascii::Ascii;
pub(super) use boolean::Boolean;
pub(super) use contains::Contains;
pub(super) use contains_any::ContainsAny;
pub(super) use contains_rune::ContainsRune;
pub(super) use ends_not_with::EndsNotWith;
pub(super) use ends_with::EndsWith;
pub(super) use excludes::Excludes;
pub(super) use excludes_all::ExcludesAll;
pub(super) use excludes_rune::ExcludesRune;
pub(super) use lowercase::Lowercase;
pub(super) use multibyte::Multibyte;
pub(super) use number::Number;
pub(super) use numeric::Numeric;
pub(super) use print_ascii::PrintAscii;
pub(super) use starts_not_with::StartsNotWith;
pub(super) use starts_with::StartsWith;
pub(super) use uppercase::Uppercase;

fn value_and_expected<'a>(
    field: &'a Field<'_>,
    expected_name: &str,
) -> Option<(Cow<'a, str>, &'a str)> {
    Some((field.value().string()?, field.params().text(expected_name)?))
}
