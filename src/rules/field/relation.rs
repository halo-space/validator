use std::cmp::Ordering;

use crate::core::{Error, Registry};
use crate::{Field, Kind, Rule, Signature, Value};

#[derive(Clone, Copy, Debug)]
enum Relation {
    Eq,
    Ne,
    Gt,
    Gte,
    Lt,
    Lte,
    Contains,
    Excludes,
}

#[derive(Debug)]
struct Compare(Relation);

impl Rule for Compare {
    fn signature(&self) -> Signature {
        Signature::text("compare").with_fields()
    }

    fn check(&self, field: &Field<'_>) -> Result<bool, Error> {
        let Some(name) = field.params().text("compare") else {
            return Ok(false);
        };
        let sibling = field.sibling(name);

        Ok(match self.0 {
            Relation::Contains => strings_contain(field.value(), sibling).unwrap_or(false),
            Relation::Excludes => {
                strings_contain(field.value(), sibling).is_none_or(|contains| !contains)
            }
            relation => compare(field.value(), sibling, relation),
        })
    }
}

pub(super) fn load(registry: &mut Registry) -> Result<(), Error> {
    registry.rule("eq_field", Compare(Relation::Eq))?;
    registry.rule("ne_field", Compare(Relation::Ne))?;
    registry.rule("gt_field", Compare(Relation::Gt))?;
    registry.rule("gte_field", Compare(Relation::Gte))?;
    registry.rule("lt_field", Compare(Relation::Lt))?;
    registry.rule("lte_field", Compare(Relation::Lte))?;
    registry.rule("fieldcontains", Compare(Relation::Contains))?;
    registry.rule("fieldexcludes", Compare(Relation::Excludes))?;
    Ok(())
}

fn compare(value: &dyn Value, sibling: Option<&dyn Value>, relation: Relation) -> bool {
    let Some(sibling) = sibling else {
        return false;
    };
    if sibling.is_none() || value.kind() != sibling.kind() {
        return false;
    }

    let ordering = ordering(value, sibling);
    match relation {
        Relation::Eq => ordering == Some(Ordering::Equal),
        Relation::Ne => ordering.is_some_and(|ordering| ordering != Ordering::Equal),
        Relation::Gt => ordering == Some(Ordering::Greater),
        Relation::Gte => ordering.is_some_and(|ordering| ordering != Ordering::Less),
        Relation::Lt => ordering == Some(Ordering::Less),
        Relation::Lte => ordering.is_some_and(|ordering| ordering != Ordering::Greater),
        Relation::Contains | Relation::Excludes => false,
    }
}

fn ordering(left: &dyn Value, right: &dyn Value) -> Option<Ordering> {
    match left.kind() {
        Kind::String => left.string()?.partial_cmp(&right.string()?),
        Kind::Bool => Some(left.boolean()?.cmp(&right.boolean()?)),
        Kind::Int(_) => Some(left.int()?.cmp(&right.int()?)),
        Kind::Uint(_) => Some(left.uint()?.cmp(&right.uint()?)),
        Kind::Float(_) => left.float()?.partial_cmp(&right.float()?),
        Kind::Vec | Kind::Array | Kind::Slice | Kind::Map => Some(left.len()?.cmp(&right.len()?)),
        Kind::Time => left.time()?.partial_cmp(&right.time()?),
        Kind::Option | Kind::Other => None,
    }
}

fn strings_contain(left: &dyn Value, right: Option<&dyn Value>) -> Option<bool> {
    Some(left.string()?.contains(right?.string()?.as_ref()))
}
