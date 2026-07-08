use crate::{Field, Rule};

#[derive(Debug)]
pub struct Ulid;

impl Rule for Ulid {
    fn check(&self, field: &Field<'_>) -> Result<bool, crate::Error> {
        Ok(field
            .value()
            .string()
            .is_some_and(|value| is_ulid(value.as_ref())))
    }
}

fn is_ulid(value: &str) -> bool {
    value.len() == 26 && value.bytes().all(is_ulid_byte)
}

fn is_ulid_byte(byte: u8) -> bool {
    let byte = byte.to_ascii_uppercase();

    byte.is_ascii_digit()
        || matches!(
            byte,
            b'A'..=b'H' | b'J'..=b'K' | b'M'..=b'N' | b'P'..=b'T' | b'V'..=b'Z'
        )
}
