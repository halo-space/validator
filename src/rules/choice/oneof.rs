use crate::{Field, Rule};

#[derive(Debug)]
pub struct OneOf;

impl Rule for OneOf {
    fn check(&self, field: &Field<'_>) -> bool {
        let Some(values) = field
            .args()
            .get("values")
            .or_else(|| field.args().get("value"))
        else {
            return false;
        };
        let Some(value) = field.value().string() else {
            return false;
        };

        values
            .split(',')
            .map(str::trim)
            .any(|candidate| !candidate.is_empty() && candidate == value.as_ref())
    }
}
