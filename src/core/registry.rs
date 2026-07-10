use std::collections::BTreeMap;
use std::sync::Arc;

use super::spec::{Expr, parse_expression};
use super::{Error, Rule};

#[derive(Clone)]
pub(crate) enum Entry {
    Rule(Arc<dyn Rule>),
    Alias(Vec<Expr>),
}

#[derive(Clone, Default)]
pub(crate) struct Registry {
    values: BTreeMap<String, Entry>,
}

impl Registry {
    pub(crate) fn new() -> Self {
        Self::default()
    }

    pub(crate) fn rule<R>(&mut self, name: impl Into<String>, rule: R) -> Result<(), Error>
    where
        R: Rule + 'static,
    {
        let name = name.into();
        validate_name(&name).map_err(|name| Error::InvalidRuleName { name })?;
        if self.contains(&name) {
            return Err(Error::DuplicateName { name });
        }
        self.values.insert(name, Entry::Rule(Arc::new(rule)));
        Ok(())
    }

    pub(crate) fn alias(
        &mut self,
        name: impl Into<String>,
        expr: impl AsRef<str>,
    ) -> Result<(), Error> {
        let name = name.into();
        validate_name(&name).map_err(|name| Error::InvalidAliasName { name })?;
        if self.contains(&name) {
            return Err(Error::DuplicateName { name });
        }
        let exprs = parse_expression(expr.as_ref())?;
        self.values.insert(name, Entry::Alias(exprs));
        Ok(())
    }

    pub(crate) fn get(&self, name: &str) -> Option<&Entry> {
        self.values.get(name)
    }

    fn contains(&self, name: &str) -> bool {
        name == "omitempty" || self.values.contains_key(name)
    }

    #[cfg(test)]
    pub(crate) fn rule_names(&self) -> impl Iterator<Item = &str> {
        self.values
            .iter()
            .filter_map(|(name, entry)| matches!(entry, Entry::Rule(_)).then_some(name.as_str()))
    }

    #[cfg(test)]
    pub(crate) fn alias_names(&self) -> impl Iterator<Item = &str> {
        self.values
            .iter()
            .filter_map(|(name, entry)| matches!(entry, Entry::Alias(_)).then_some(name.as_str()))
    }
}

fn validate_name(name: &str) -> Result<(), String> {
    let valid = !name.is_empty()
        && name
            .chars()
            .all(|ch| ch.is_ascii_alphanumeric() || ch == '_' || ch == '-');

    if valid { Ok(()) } else { Err(name.to_owned()) }
}
