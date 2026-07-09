use crate::{Field, Rule};

#[derive(Debug)]
pub struct DateTime;

impl Rule for DateTime {
    fn check(&self, field: &Field<'_>) -> Result<bool, crate::Error> {
        Ok(field
            .value()
            .string()
            .is_some_and(|value| valid(value.as_ref())))
    }
}

fn valid(value: &str) -> bool {
    let Some((date, time)) = value.split_once('T') else {
        return false;
    };

    valid_date(date) && valid_time_with_offset(time)
}

fn valid_date(value: &str) -> bool {
    let parts = value.split('-').collect::<Vec<_>>();
    if parts.len() != 3 {
        return false;
    }

    let Ok(year) = parts[0].parse::<u16>() else {
        return false;
    };
    let Ok(month) = parts[1].parse::<u8>() else {
        return false;
    };
    let Ok(day) = parts[2].parse::<u8>() else {
        return false;
    };

    parts[0].len() == 4
        && parts[1].len() == 2
        && parts[2].len() == 2
        && year > 0
        && (1..=12).contains(&month)
        && (1..=days_in_month(year, month)).contains(&day)
}

fn valid_time_with_offset(value: &str) -> bool {
    if let Some(time) = value.strip_suffix('Z') {
        return valid_time(time);
    }

    let Some((time, offset)) = split_offset(value) else {
        return false;
    };

    valid_time(time) && valid_offset(offset)
}

fn split_offset(value: &str) -> Option<(&str, &str)> {
    let index = value.rfind(['+', '-'])?;
    if index < 8 {
        return None;
    }

    Some(value.split_at(index))
}

fn valid_time(value: &str) -> bool {
    let (main, fraction) = value.split_once('.').unwrap_or((value, ""));
    let parts = main.split(':').collect::<Vec<_>>();
    if parts.len() != 3 {
        return false;
    }

    let Ok(hour) = parts[0].parse::<u8>() else {
        return false;
    };
    let Ok(minute) = parts[1].parse::<u8>() else {
        return false;
    };
    let Ok(second) = parts[2].parse::<u8>() else {
        return false;
    };

    parts[0].len() == 2
        && parts[1].len() == 2
        && parts[2].len() == 2
        && hour <= 23
        && minute <= 59
        && second <= 60
        && (fraction.is_empty() || fraction.bytes().all(|byte| byte.is_ascii_digit()))
}

fn valid_offset(value: &str) -> bool {
    let Some(sign) = value.as_bytes().first() else {
        return false;
    };
    if !matches!(sign, b'+' | b'-') {
        return false;
    }

    let parts = value[1..].split(':').collect::<Vec<_>>();
    if parts.len() != 2 || parts[0].len() != 2 || parts[1].len() != 2 {
        return false;
    }

    let Ok(hour) = parts[0].parse::<u8>() else {
        return false;
    };
    let Ok(minute) = parts[1].parse::<u8>() else {
        return false;
    };

    hour <= 23 && minute <= 59
}

fn days_in_month(year: u16, month: u8) -> u8 {
    match month {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 if leap_year(year) => 29,
        2 => 28,
        _ => 0,
    }
}

fn leap_year(year: u16) -> bool {
    year.is_multiple_of(4) && !year.is_multiple_of(100) || year.is_multiple_of(400)
}
