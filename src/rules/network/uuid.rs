use crate::{Field, Rule};

#[derive(Debug)]
pub struct Uuid;

impl Rule for Uuid {
    fn check(&self, field: &Field<'_>) -> bool {
        field
            .value()
            .string()
            .is_some_and(|value| is_uuid(value.as_ref()))
    }
}

#[derive(Debug)]
pub struct Uuid3;

impl Rule for Uuid3 {
    fn check(&self, field: &Field<'_>) -> bool {
        field
            .value()
            .string()
            .is_some_and(|value| is_uuid_version(value.as_ref(), b'3'))
    }
}

#[derive(Debug)]
pub struct Uuid4;

impl Rule for Uuid4 {
    fn check(&self, field: &Field<'_>) -> bool {
        field
            .value()
            .string()
            .is_some_and(|value| is_uuid_version(value.as_ref(), b'4'))
    }
}

#[derive(Debug)]
pub struct Uuid5;

impl Rule for Uuid5 {
    fn check(&self, field: &Field<'_>) -> bool {
        field
            .value()
            .string()
            .is_some_and(|value| is_uuid_version(value.as_ref(), b'5'))
    }
}

fn is_uuid(value: &str) -> bool {
    uuid_version(value).is_some()
}

fn is_uuid_version(value: &str, version: u8) -> bool {
    uuid_version(value) == Some(version)
}

fn uuid_version(value: &str) -> Option<u8> {
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
