use std::sync::OnceLock;

use regex::Regex;

use crate::{Field, Rule};

#[derive(Debug)]
pub struct Mongodb;

#[derive(Debug)]
pub struct MongodbConnectionString;

impl Rule for Mongodb {
    fn check(&self, field: &Field<'_>) -> Result<bool, crate::Error> {
        Ok(field
            .value()
            .string()
            .is_some_and(|value| object_id_pattern().is_match(value.as_ref())))
    }
}

impl Rule for MongodbConnectionString {
    fn check(&self, field: &Field<'_>) -> Result<bool, crate::Error> {
        Ok(field
            .value()
            .string()
            .is_some_and(|value| connection_pattern().is_match(value.as_ref())))
    }
}

fn object_id_pattern() -> &'static Regex {
    static PATTERN: OnceLock<Regex> = OnceLock::new();
    PATTERN.get_or_init(|| Regex::new(r"^[a-f0-9]{24}$").expect("mongodb regex must compile"))
}

fn connection_pattern() -> &'static Regex {
    static PATTERN: OnceLock<Regex> = OnceLock::new();
    PATTERN.get_or_init(|| {
        Regex::new(r"^mongodb(\+srv)?://(([a-zA-Z0-9]+):([a-zA-Z0-9$:/?#\[\]@]+)@)?(([a-z0-9.-]+)(:[0-9]+)?)((,(([a-z0-9.-]+)(:([0-9]+))?))*)?(/[a-zA-Z-_]{1,64})?(\?(([a-zA-Z]+)=([a-zA-Z0-9]+))(&(([a-zA-Z0-9]+)=([a-zA-Z0-9]+))?)*)?$")
            .expect("mongodb_connection_string regex must compile")
    })
}
