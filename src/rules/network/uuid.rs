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

fn is_uuid(value: &str) -> bool {
    if value.len() != 36 {
        return false;
    }

    value.bytes().enumerate().all(|(index, byte)| match index {
        8 | 13 | 18 | 23 => byte == b'-',
        _ => byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte),
    })
}
