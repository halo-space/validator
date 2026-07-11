use crate::{Field, Rule};

#[derive(Debug)]
pub(crate) struct Isbn;

#[derive(Debug)]
pub(crate) struct Isbn10;

#[derive(Debug)]
pub(crate) struct Isbn13;

#[derive(Debug)]
pub(crate) struct Issn;

impl Rule for Isbn {
    fn check(&self, field: &Field<'_>) -> Result<bool, crate::Error> {
        Ok(field.value().string().is_some_and(|value| {
            let value = value.as_ref();
            valid_isbn10(value) || valid_isbn13(value)
        }))
    }
}

impl Rule for Isbn10 {
    fn check(&self, field: &Field<'_>) -> Result<bool, crate::Error> {
        Ok(field
            .value()
            .string()
            .is_some_and(|value| valid_isbn10(value.as_ref())))
    }
}

impl Rule for Isbn13 {
    fn check(&self, field: &Field<'_>) -> Result<bool, crate::Error> {
        Ok(field
            .value()
            .string()
            .is_some_and(|value| valid_isbn13(value.as_ref())))
    }
}

impl Rule for Issn {
    fn check(&self, field: &Field<'_>) -> Result<bool, crate::Error> {
        Ok(field
            .value()
            .string()
            .is_some_and(|value| valid_issn(value.as_ref())))
    }
}

fn valid_isbn10(value: &str) -> bool {
    let value = cleaned(value, 3);
    let bytes = value.as_bytes();

    if bytes.len() != 10 || !bytes[..9].iter().all(u8::is_ascii_digit) {
        return false;
    }
    if !bytes[9].is_ascii_digit() && bytes[9] != b'X' {
        return false;
    }

    let mut checksum = 0_u32;
    for (index, byte) in bytes[..9].iter().enumerate() {
        checksum += (index as u32 + 1) * u32::from(byte - b'0');
    }

    checksum += 10
        * if bytes[9] == b'X' {
            10
        } else {
            u32::from(bytes[9] - b'0')
        };

    checksum.is_multiple_of(11)
}

fn valid_isbn13(value: &str) -> bool {
    let value = cleaned(value, 4);
    let bytes = value.as_bytes();

    if bytes.len() != 13
        || !bytes.iter().all(u8::is_ascii_digit)
        || !(value.starts_with("978") || value.starts_with("979"))
    {
        return false;
    }

    let checksum = bytes[..12]
        .iter()
        .enumerate()
        .map(|(index, byte)| {
            let digit = u32::from(byte - b'0');
            if index % 2 == 0 { digit } else { digit * 3 }
        })
        .sum::<u32>();
    let check_digit = (10 - (checksum % 10)) % 10;

    u32::from(bytes[12] - b'0') == check_digit
}

fn valid_issn(value: &str) -> bool {
    let bytes = value.as_bytes();
    if bytes.len() != 9
        || bytes[4] != b'-'
        || !bytes[..4].iter().all(u8::is_ascii_digit)
        || !bytes[5..8].iter().all(u8::is_ascii_digit)
        || (!bytes[8].is_ascii_digit() && bytes[8] != b'X')
    {
        return false;
    }

    let digits = value.replace('-', "");
    let bytes = digits.as_bytes();
    let checksum = bytes[..7]
        .iter()
        .enumerate()
        .map(|(index, byte)| (8 - index as u32) * u32::from(byte - b'0'))
        .sum::<u32>()
        + if bytes[7] == b'X' {
            10
        } else {
            u32::from(bytes[7] - b'0')
        };

    checksum.is_multiple_of(11)
}

fn cleaned(value: &str, max_separators: usize) -> String {
    let mut output = String::with_capacity(value.len());
    let mut removed = 0;

    for byte in value.bytes() {
        if (byte == b'-' || byte == b' ') && removed < max_separators {
            removed += 1;
        } else {
            output.push(byte as char);
        }
    }

    output
}
