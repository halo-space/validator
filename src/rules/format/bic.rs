use std::sync::OnceLock;

use regex::Regex;

use crate::{Field, Rule};

#[derive(Debug)]
pub struct Bic;

#[derive(Debug)]
pub struct BicIso93622014;

impl Rule for Bic {
    fn check(&self, field: &Field<'_>) -> Result<bool, crate::Error> {
        Ok(field
            .value()
            .string()
            .is_some_and(|value| bic_2022().is_match(value.as_ref())))
    }
}

impl Rule for BicIso93622014 {
    fn check(&self, field: &Field<'_>) -> Result<bool, crate::Error> {
        Ok(field
            .value()
            .string()
            .is_some_and(|value| bic_2014().is_match(value.as_ref())))
    }
}

fn bic_2014() -> &'static Regex {
    static PATTERN: OnceLock<Regex> = OnceLock::new();
    PATTERN.get_or_init(|| {
        Regex::new(r"^[A-Za-z]{6}[A-Za-z0-9]{2}([A-Za-z0-9]{3})?$")
            .expect("bic_iso_9362_2014 regex must compile")
    })
}

fn bic_2022() -> &'static Regex {
    static PATTERN: OnceLock<Regex> = OnceLock::new();
    PATTERN.get_or_init(|| {
        Regex::new(r"^[A-Z0-9]{4}[A-Z]{2}[A-Z0-9]{2}(?:[A-Z0-9]{3})?$")
            .expect("bic regex must compile")
    })
}
