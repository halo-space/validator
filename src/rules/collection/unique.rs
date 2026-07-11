use std::collections::BTreeSet;

use crate::{Error, Field, Kind, Rule, Signature};

#[derive(Debug)]
pub struct Unique;

impl Rule for Unique {
    fn signature(&self) -> Signature {
        Signature::optional_field_list("fields")
    }

    fn validate_params(&self, field: &Field<'_>) -> Result<(), Error> {
        let kind = field.value().kind();
        let fields = field.params().list("fields");
        if let Some(fields) = fields {
            let mut seen = BTreeSet::new();
            if let Some(duplicate) = fields.iter().find(|name| !seen.insert(name.as_str())) {
                return Err(Error::InvalidRuleExpression {
                    expression: "unique".to_owned(),
                    reason: format!("duplicate field path '{duplicate}'"),
                });
            }
        }

        let supported = if fields.is_some() {
            matches!(kind, Kind::Vec | Kind::Array | Kind::Slice | Kind::Option)
        } else {
            matches!(
                kind,
                Kind::Vec | Kind::Array | Kind::Slice | Kind::Map | Kind::Option
            )
        };

        if supported {
            Ok(())
        } else {
            Err(invalid_kind(kind, fields.is_some()))
        }
    }

    fn check(&self, field: &Field<'_>) -> Result<bool, crate::Error> {
        if let Some(fields) = field.params().list("fields") {
            if !matches!(field.value().kind(), Kind::Vec | Kind::Array | Kind::Slice) {
                return Err(invalid_kind(field.value().kind(), true));
            }
            let items = field.items().ok_or_else(|| Error::MissingFieldContext {
                name: "unique".to_owned(),
            })?;
            return super::fields_are_unique(items, fields);
        }

        let items = field
            .value()
            .array_items()
            .or_else(|| field.value().map_values())
            .ok_or_else(|| invalid_kind(field.value().kind(), false))?;
        super::values_are_unique(items)
    }
}

fn invalid_kind(kind: Kind, projection: bool) -> Error {
    let reason = if projection {
        format!("field projection requires a Vec, array, or slice; found {kind:?}")
    } else {
        format!("field kind {kind:?} does not support collection uniqueness")
    };

    Error::InvalidRuleExpression {
        expression: "unique".to_owned(),
        reason,
    }
}
