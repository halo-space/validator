use crate::{Field, Rule};

#[derive(Debug)]
pub struct Mac;

impl Rule for Mac {
    fn check(&self, field: &Field<'_>) -> Result<bool, crate::Error> {
        Ok(field
            .value()
            .string()
            .is_some_and(|value| valid(value.as_ref())))
    }
}

fn valid(value: &str) -> bool {
    valid_split(value, ':', 6, 2) || valid_split(value, '-', 6, 2) || valid_split(value, '.', 3, 4)
}

fn valid_split(value: &str, separator: char, expected_parts: usize, width: usize) -> bool {
    let parts = value.split(separator).collect::<Vec<_>>();
    parts.len() == expected_parts
        && parts.len()
            == parts
                .iter()
                .filter(|part| {
                    part.len() == width && part.bytes().all(|byte| byte.is_ascii_hexdigit())
                })
                .count()
}
