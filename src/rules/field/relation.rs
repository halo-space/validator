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
        Signature::field("compare")
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

    match relation {
        Relation::Eq => equality(value, sibling).unwrap_or(false),
        Relation::Ne => equality(value, sibling).is_some_and(|equal| !equal),
        Relation::Gt => ordering(value, sibling) == Some(Ordering::Greater),
        Relation::Gte => {
            ordering(value, sibling).is_some_and(|ordering| ordering != Ordering::Less)
        }
        Relation::Lt => ordering(value, sibling) == Some(Ordering::Less),
        Relation::Lte => {
            ordering(value, sibling).is_some_and(|ordering| ordering != Ordering::Greater)
        }
        Relation::Contains | Relation::Excludes => false,
    }
}

fn equality(left: &dyn Value, right: &dyn Value) -> Option<bool> {
    match left.kind() {
        Kind::String => Some(left.string()? == right.string()?),
        Kind::Bool => Some(left.boolean()? == right.boolean()?),
        Kind::Int(_) => Some(left.int()? == right.int()?),
        Kind::Uint(_) => Some(left.uint()? == right.uint()?),
        Kind::Float(_) => Some(left.float()? == right.float()?),
        Kind::Vec | Kind::Array | Kind::Slice | Kind::Map => Some(left.len()? == right.len()?),
        Kind::Time => Some(left.time()? == right.time()?),
        Kind::Option | Kind::Other => None,
    }
}

fn ordering(left: &dyn Value, right: &dyn Value) -> Option<Ordering> {
    match left.kind() {
        Kind::String => Some(left.len()?.cmp(&right.len()?)),
        Kind::Int(_) => Some(left.int()?.cmp(&right.int()?)),
        Kind::Uint(_) => Some(left.uint()?.cmp(&right.uint()?)),
        Kind::Float(_) => left.float()?.partial_cmp(&right.float()?),
        Kind::Vec | Kind::Array | Kind::Slice | Kind::Map => Some(left.len()?.cmp(&right.len()?)),
        Kind::Time => left.time()?.partial_cmp(&right.time()?),
        Kind::Bool | Kind::Option | Kind::Other => None,
    }
}

fn strings_contain(left: &dyn Value, right: Option<&dyn Value>) -> Option<bool> {
    Some(left.string()?.contains(right?.string()?.as_ref()))
}
