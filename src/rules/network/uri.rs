use crate::{Field, Rule};

#[derive(Debug)]
pub struct Uri;

impl Rule for Uri {
    fn check(&self, field: &Field<'_>) -> Result<bool, crate::Error> {
        Ok(field
            .value()
            .string()
            .is_some_and(|value| valid(value.as_ref())))
    }
}

fn valid(value: &str) -> bool {
    let Some((scheme, rest)) = value.split_once(':') else {
        return false;
    };

    !scheme.is_empty()
        && valid_scheme(scheme)
        && !rest.is_empty()
        && !rest.chars().any(char::is_whitespace)
}

fn valid_scheme(value: &str) -> bool {
    let mut chars = value.chars();
    let Some(first) = chars.next() else {
        return false;
    };

    first.is_ascii_alphabetic()
        && chars.all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '+' | '-' | '.'))
}
