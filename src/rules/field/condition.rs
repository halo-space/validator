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
            | Condition::ExcludedUnless => Signature::pairs("conditions").with_fields(),
            Condition::RequiredWith
            | Condition::RequiredWithAll
            | Condition::RequiredWithout
            | Condition::RequiredWithoutAll
            | Condition::ExcludedWith
            | Condition::ExcludedWithAll
            | Condition::ExcludedWithout
            | Condition::ExcludedWithoutAll => Signature::list("fields").with_fields(),
        }
    }

    fn validates_none(&self) -> bool {
        true
    }

    fn check(&self, field: &Field<'_>) -> Result<bool, Error> {
        let required = field.value().required();
        let matches_all = || {
            field
                .params()
                .pairs("conditions")
                .into_iter()
                .flatten()
                .all(|(name, expected)| matches(field.sibling(name), expected))
        };
        let matches_any = || {
            field
                .params()
                .pairs("conditions")
                .into_iter()
                .flatten()
                .any(|(name, expected)| matches(field.sibling(name), expected))
        };
        let any_satisfies_required =
            || fields(field).any(|value| value.is_some_and(Value::required));
        let all_satisfy_required = || fields(field).all(|value| value.is_some_and(Value::required));
        let any_fails_required = || fields(field).any(|value| !value.is_some_and(Value::required));
        let all_fail_required = || fields(field).all(|value| !value.is_some_and(Value::required));

        Ok(match self.0 {
            Condition::RequiredIf | Condition::SkipUnless => !matches_all() || required,
            Condition::RequiredUnless => matches_any() || required,
            Condition::RequiredWith => !any_satisfies_required() || required,
            Condition::RequiredWithAll => !all_satisfy_required() || required,
            Condition::RequiredWithout => !any_fails_required() || required,
            Condition::RequiredWithoutAll => !all_fail_required() || required,
            Condition::ExcludedIf => !matches_all() || !required,
            Condition::ExcludedUnless => matches_any() || !required,
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

fn matches(value: Option<&dyn Value>, expected: &str) -> bool {
    let Some(value) = value else {
        return false;
    };
    if value.is_none() {
        return expected == "null";
    }

    match value.kind() {
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
    }
}
