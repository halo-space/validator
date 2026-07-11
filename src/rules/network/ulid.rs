use crate::{Field, Rule};

#[derive(Debug)]
pub(crate) struct Ulid;

impl Rule for Ulid {
    fn check(&self, field: &Field<'_>) -> Result<bool, crate::Error> {
        Ok(field
            .value()
            .string()
            .is_some_and(|value| valid(value.as_ref())))
    }
}

fn valid(value: &str) -> bool {
    value.len() == 26 && value.bytes().all(valid_byte)
}

fn valid_byte(byte: u8) -> bool {
    let byte = byte.to_ascii_uppercase();

    byte.is_ascii_digit()
        || matches!(
            byte,
            b'A'..=b'H' | b'J'..=b'K' | b'M'..=b'N' | b'P'..=b'T' | b'V'..=b'Z'
        )
}
