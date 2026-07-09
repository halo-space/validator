use crate::{Field, Rule};

#[derive(Debug)]
pub struct Md4;

#[derive(Debug)]
pub struct Md5;

#[derive(Debug)]
pub struct Sha256;

#[derive(Debug)]
pub struct Sha384;

#[derive(Debug)]
pub struct Sha512;

#[derive(Debug)]
pub struct Ripemd128;

#[derive(Debug)]
pub struct Ripemd160;

#[derive(Debug)]
pub struct Tiger128;

#[derive(Debug)]
pub struct Tiger160;

#[derive(Debug)]
pub struct Tiger192;

macro_rules! hash_rule {
    ($ty:ty, $len:expr) => {
        impl Rule for $ty {
            fn check(&self, field: &Field<'_>) -> Result<bool, crate::Error> {
                Ok(field
                    .value()
                    .string()
                    .is_some_and(|value| valid(value.as_ref(), $len)))
            }
        }
    };
}

hash_rule!(Md4, 32);
hash_rule!(Md5, 32);
hash_rule!(Sha256, 64);
hash_rule!(Sha384, 96);
hash_rule!(Sha512, 128);
hash_rule!(Ripemd128, 32);
hash_rule!(Ripemd160, 40);
hash_rule!(Tiger128, 32);
hash_rule!(Tiger160, 40);
hash_rule!(Tiger192, 48);

fn valid(value: &str, len: usize) -> bool {
    value.len() == len
        && value
            .bytes()
            .all(|byte| byte.is_ascii_hexdigit() && !byte.is_ascii_uppercase())
}
