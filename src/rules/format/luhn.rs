use crate::{Field, Rule};

#[derive(Debug)]
pub(crate) struct CreditCard;

#[derive(Debug)]
pub(crate) struct LuhnChecksum;

impl Rule for CreditCard {
    fn check(&self, field: &Field<'_>) -> Result<bool, crate::Error> {
        Ok(field
            .value()
            .string()
            .is_some_and(|value| valid_credit_card(value.as_ref())))
    }
}

impl Rule for LuhnChecksum {
    fn check(&self, field: &Field<'_>) -> Result<bool, crate::Error> {
        Ok(luhn_value(field).is_some_and(|value| valid_luhn(value.as_bytes())))
    }
}

fn valid_credit_card(value: &str) -> bool {
    let mut digits = String::with_capacity(value.len());

    for segment in value.split(' ') {
        if segment.len() < 3 {
            return false;
        }
        digits.push_str(segment);
    }

    (12..=19).contains(&digits.len()) && valid_luhn(digits.as_bytes())
}

fn luhn_value(field: &Field<'_>) -> Option<String> {
    if let Some(value) = field.value().string() {
        return Some(value.into_owned());
    }

    if let Some(value) = field.value().int() {
        if value < 0 {
            return None;
        }
        return Some(value.to_string());
    }

    field.value().uint().map(|value| value.to_string())
}

fn valid_luhn(digits: &[u8]) -> bool {
    if digits.len() < 2 || !digits.iter().all(u8::is_ascii_digit) {
        return false;
    }

    let size = digits.len();
    let mut sum = 0_u32;

    for (index, byte) in digits.iter().enumerate() {
        let value = u32::from(byte - b'0');
        if (size.is_multiple_of(2) && index.is_multiple_of(2))
            || (!size.is_multiple_of(2) && !index.is_multiple_of(2))
        {
            let doubled = value * 2;
            sum += if doubled >= 10 {
                1 + (doubled % 10)
            } else {
                doubled
            };
        } else {
            sum += value;
        }
    }

    sum.is_multiple_of(10)
}
