use std::sync::OnceLock;

use regex::Regex;

use crate::{Field, Rule};

#[derive(Debug)]
pub struct HexColor;

impl Rule for HexColor {
    fn check(&self, field: &Field<'_>) -> bool {
        field
            .value()
            .string()
            .is_some_and(|value| hexcolor_regex().is_match(value.as_ref()))
    }
}

fn hexcolor_regex() -> &'static Regex {
    static HEXCOLOR: OnceLock<Regex> = OnceLock::new();
    HEXCOLOR.get_or_init(|| {
        Regex::new(r"^#(?:[0-9a-fA-F]{3}|[0-9a-fA-F]{4}|[0-9a-fA-F]{6}|[0-9a-fA-F]{8})$")
            .expect("hexcolor regex must compile")
    })
}
