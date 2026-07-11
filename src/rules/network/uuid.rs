use crate::{Field, Rule};

#[derive(Debug)]
pub(crate) struct Uuid;

impl Rule for Uuid {
    fn check(&self, field: &Field<'_>) -> Result<bool, crate::Error> {
        Ok(field
            .value()
            .string()
            .is_some_and(|value| valid(value.as_ref())))
    }
}

#[derive(Debug)]
pub(crate) struct Uuid3;

impl Rule for Uuid3 {
    fn check(&self, field: &Field<'_>) -> Result<bool, crate::Error> {
        Ok(field
            .value()
            .string()
            .is_some_and(|value| valid_version(value.as_ref(), b'3')))
    }
}

#[derive(Debug)]
pub(crate) struct Uuid4;

impl Rule for Uuid4 {
    fn check(&self, field: &Field<'_>) -> Result<bool, crate::Error> {
        Ok(field
            .value()
            .string()
            .is_some_and(|value| valid_version_and_variant(value.as_ref(), b'4')))
    }
}

#[derive(Debug)]
pub(crate) struct Uuid5;

#[derive(Debug)]
pub(crate) struct UuidRfc4122;

#[derive(Debug)]
pub(crate) struct Uuid3Rfc4122;

#[derive(Debug)]
pub(crate) struct Uuid4Rfc4122;

#[derive(Debug)]
pub(crate) struct Uuid5Rfc4122;

impl Rule for Uuid5 {
    fn check(&self, field: &Field<'_>) -> Result<bool, crate::Error> {
        Ok(field
            .value()
            .string()
            .is_some_and(|value| valid_version_and_variant(value.as_ref(), b'5')))
    }
}

impl Rule for UuidRfc4122 {
    fn check(&self, field: &Field<'_>) -> Result<bool, crate::Error> {
        Ok(field
            .value()
            .string()
            .is_some_and(|value| valid_rfc(value.as_ref())))
    }
}

impl Rule for Uuid3Rfc4122 {
    fn check(&self, field: &Field<'_>) -> Result<bool, crate::Error> {
        Ok(field
            .value()
            .string()
            .is_some_and(|value| valid_rfc_version(value.as_ref(), b'3')))
    }
}

impl Rule for Uuid4Rfc4122 {
    fn check(&self, field: &Field<'_>) -> Result<bool, crate::Error> {
        Ok(field
            .value()
            .string()
            .is_some_and(|value| valid_rfc_version_and_variant(value.as_ref(), b'4')))
    }
}

impl Rule for Uuid5Rfc4122 {
    fn check(&self, field: &Field<'_>) -> Result<bool, crate::Error> {
        Ok(field
            .value()
            .string()
            .is_some_and(|value| valid_rfc_version_and_variant(value.as_ref(), b'5')))
    }
}

fn valid(value: &str) -> bool {
    version(value).is_some()
}

fn valid_version(value: &str, expected: u8) -> bool {
    version(value) == Some(expected)
}

fn valid_version_and_variant(value: &str, expected: u8) -> bool {
    version(value) == Some(expected)
        && value
            .as_bytes()
            .get(19)
            .is_some_and(|byte| matches!(byte, b'8' | b'9' | b'a' | b'b'))
}

fn valid_rfc(value: &str) -> bool {
    rfc_version(value).is_some()
}

fn valid_rfc_version(value: &str, expected: u8) -> bool {
    rfc_version(value) == Some(expected)
}

fn valid_rfc_version_and_variant(value: &str, expected: u8) -> bool {
    rfc_version(value) == Some(expected)
        && value
            .as_bytes()
            .get(19)
            .is_some_and(|byte| matches!(byte, b'8' | b'9' | b'a' | b'A' | b'b' | b'B'))
}

fn version(value: &str) -> Option<u8> {
    if value.len() != 36 {
        return None;
    }

    let valid = value.bytes().enumerate().all(|(index, byte)| match index {
        8 | 13 | 18 | 23 => byte == b'-',
        _ => byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte),
    });

    if valid {
        value.as_bytes().get(14).copied()
    } else {
        None
    }
}

fn rfc_version(value: &str) -> Option<u8> {
    if value.len() != 36 {
        return None;
    }

    let valid = value.bytes().enumerate().all(|(index, byte)| match index {
        8 | 13 | 18 | 23 => byte == b'-',
        _ => byte.is_ascii_hexdigit(),
    });

    if valid {
        value.as_bytes().get(14).copied()
    } else {
        None
    }
}
