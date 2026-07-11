use crate::core::{Error, Registry};
use crate::{Field, Kind, Rule, Signature, Value};

#[derive(Clone, Copy, Debug)]
enum Condition {
    RequiredIf,
    RequiredUnless,
    SkipUnless,
    RequiredWith,
    RequiredWithAll,
    RequiredWithout,
    RequiredWithoutAll,
    ExcludedIf,
    ExcludedUnless,
    ExcludedWith,
    ExcludedWithAll,
    ExcludedWithout,
    ExcludedWithoutAll,
}

#[derive(Debug)]
struct Check(Condition);

impl Rule for Check {
    fn signature(&self) -> Signature {
        match self.0 {
            Condition::RequiredIf
            | Condition::RequiredUnless
            | Condition::SkipUnless
            | Condition::ExcludedIf
            | Condition::ExcludedUnless => Signature::field_pairs("conditions"),
            Condition::RequiredWith
            | Condition::RequiredWithAll
            | Condition::RequiredWithout
            | Condition::RequiredWithoutAll
            | Condition::ExcludedWith
            | Condition::ExcludedWithAll
            | Condition::ExcludedWithout
            | Condition::ExcludedWithoutAll => Signature::field_list("fields"),
        }
    }

    fn validates_none(&self) -> bool {
        true
    }

    fn validate_params(&self, field: &Field<'_>) -> Result<(), Error> {
        if matches!(
            self.0,
            Condition::RequiredIf
                | Condition::RequiredUnless
                | Condition::SkipUnless
                | Condition::ExcludedIf
                | Condition::ExcludedUnless
        ) {
            validate_conditions(field, self.0)?;
        }
        Ok(())
    }

    fn check(&self, field: &Field<'_>) -> Result<bool, Error> {
        let required = field.value().required();
        let any_satisfies_required =
            || fields(field).any(|value| value.is_some_and(Value::required));
        let all_satisfy_required = || fields(field).all(|value| value.is_some_and(Value::required));
        let any_fails_required = || fields(field).any(|value| !value.is_some_and(Value::required));
        let all_fail_required = || fields(field).all(|value| !value.is_some_and(Value::required));

        Ok(match self.0 {
            Condition::RequiredIf | Condition::SkipUnless => {
                !conditions_match(field, self.0, true)? || required
            }
            Condition::RequiredUnless => conditions_match(field, self.0, false)? || required,
            Condition::RequiredWith => !any_satisfies_required() || required,
            Condition::RequiredWithAll => !all_satisfy_required() || required,
            Condition::RequiredWithout => !any_fails_required() || required,
            Condition::RequiredWithoutAll => !all_fail_required() || required,
            Condition::ExcludedIf => !conditions_match(field, self.0, true)? || !required,
            Condition::ExcludedUnless => conditions_match(field, self.0, false)? || !required,
            Condition::ExcludedWith => !any_satisfies_required() || !required,
            Condition::ExcludedWithAll => !all_satisfy_required() || !required,
            Condition::ExcludedWithout => !any_fails_required() || !required,
            Condition::ExcludedWithoutAll => !all_fail_required() || !required,
        })
    }
}

pub(super) fn load(registry: &mut Registry) -> Result<(), Error> {
    registry.rule("required_if", Check(Condition::RequiredIf))?;
    registry.rule("required_unless", Check(Condition::RequiredUnless))?;
    registry.rule("skip_unless", Check(Condition::SkipUnless))?;
    registry.rule("required_with", Check(Condition::RequiredWith))?;
    registry.rule("required_with_all", Check(Condition::RequiredWithAll))?;
    registry.rule("required_without", Check(Condition::RequiredWithout))?;
    registry.rule("required_without_all", Check(Condition::RequiredWithoutAll))?;
    registry.rule("excluded_if", Check(Condition::ExcludedIf))?;
    registry.rule("excluded_unless", Check(Condition::ExcludedUnless))?;
    registry.rule("excluded_with", Check(Condition::ExcludedWith))?;
    registry.rule("excluded_with_all", Check(Condition::ExcludedWithAll))?;
    registry.rule("excluded_without", Check(Condition::ExcludedWithout))?;
    registry.rule("excluded_without_all", Check(Condition::ExcludedWithoutAll))?;
    Ok(())
}

fn fields<'a>(field: &'a Field<'a>) -> impl Iterator<Item = Option<&'a dyn Value>> + 'a {
    field
        .params()
        .list("fields")
        .into_iter()
        .flatten()
        .map(|name| field.sibling(name))
}

fn validate_conditions(field: &Field<'_>, condition: Condition) -> Result<(), Error> {
    for (name, expected) in field.params().pairs("conditions").into_iter().flatten() {
        validate_expected(condition, name, field.sibling(name), expected)?;
    }
    Ok(())
}

fn conditions_match(field: &Field<'_>, condition: Condition, all: bool) -> Result<bool, Error> {
    let pairs = field.params().pairs("conditions").into_iter().flatten();
    if all {
        for (name, expected) in pairs {
            if !matches_value(condition, name, field.sibling(name), expected)? {
                return Ok(false);
            }
        }
        Ok(true)
    } else {
        for (name, expected) in pairs {
            if matches_value(condition, name, field.sibling(name), expected)? {
                return Ok(true);
            }
        }
        Ok(false)
    }
}

fn matches_value(
    condition: Condition,
    name: &str,
    value: Option<&dyn Value>,
    expected: &str,
) -> Result<bool, Error> {
    validate_expected(condition, name, value, expected)?;
    let Some(value) = value else {
        return Ok(false);
    };
    if value.is_none() {
        return Ok(expected == "null");
    }

    Ok(match value.kind() {
        Kind::String => value.string().is_some_and(|value| value == expected),
        Kind::Bool => expected
            .parse::<bool>()
            .is_ok_and(|expected| value.boolean() == Some(expected)),
        Kind::Int(_) => expected
            .parse::<i128>()
            .is_ok_and(|expected| value.int() == Some(expected)),
        Kind::Uint(_) => expected
            .parse::<u128>()
            .is_ok_and(|expected| value.uint() == Some(expected)),
        Kind::Float(_) => expected
            .parse::<f64>()
            .is_ok_and(|expected| value.float() == Some(expected)),
        Kind::Vec | Kind::Array | Kind::Slice | Kind::Map => expected
            .parse::<usize>()
            .is_ok_and(|expected| value.len() == Some(expected)),
        Kind::Time | Kind::Option | Kind::Other => false,
    })
}

fn validate_expected(
    condition: Condition,
    name: &str,
    value: Option<&dyn Value>,
    expected: &str,
) -> Result<(), Error> {
    if expected == "null" {
        return Ok(());
    }

    let Some(value) = value else {
        return Ok(());
    };
    let expected_type = match value.kind() {
        Kind::String | Kind::Time | Kind::Option | Kind::Other => return Ok(()),
        Kind::Bool => expected.parse::<bool>().map(|_| ()).map_err(|_| "boolean"),
        Kind::Int(_) => expected.parse::<i128>().map(|_| ()).map_err(|_| "int"),
        Kind::Uint(_) => expected.parse::<u128>().map(|_| ()).map_err(|_| "uint"),
        Kind::Float(_) => expected.parse::<f64>().map(|_| ()).map_err(|_| "float"),
        Kind::Vec | Kind::Array | Kind::Slice | Kind::Map => expected
            .parse::<usize>()
            .map(|_| ())
            .map_err(|_| "collection length"),
    };

    expected_type.map_err(|expected_type| Error::InvalidRuleExpression {
        expression: condition.name().to_owned(),
        reason: format!(
            "condition for field '{name}' must be a valid {expected_type}, got '{expected}'"
        ),
    })
}

impl Condition {
    fn name(self) -> &'static str {
        match self {
            Self::RequiredIf => "required_if",
            Self::RequiredUnless => "required_unless",
            Self::SkipUnless => "skip_unless",
            Self::RequiredWith => "required_with",
            Self::RequiredWithAll => "required_with_all",
            Self::RequiredWithout => "required_without",
            Self::RequiredWithoutAll => "required_without_all",
            Self::ExcludedIf => "excluded_if",
            Self::ExcludedUnless => "excluded_unless",
            Self::ExcludedWith => "excluded_with",
            Self::ExcludedWithAll => "excluded_with_all",
            Self::ExcludedWithout => "excluded_without",
            Self::ExcludedWithoutAll => "excluded_without_all",
        }
    }
}
