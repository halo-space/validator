use std::collections::BTreeMap;
use std::sync::Arc;

use super::spec::{Expr, parse_expression};
use super::{Error, Rule};

#[derive(Clone, Default)]
pub(crate) struct Rules {
    values: BTreeMap<String, Arc<dyn Rule>>,
}

impl Rules {
    pub(crate) fn new() -> Self {
        Self::default()
    }

    pub(crate) fn insert<R>(&mut self, name: impl Into<String>, rule: R) -> Result<(), Error>
    where
        R: Rule + 'static,
    {
        let name = name.into();
        validate_name(&name).map_err(|name| Error::InvalidRuleName { name })?;
        self.values.insert(name, Arc::new(rule));
        Ok(())
    }

    pub(crate) fn get(&self, name: &str) -> Option<Arc<dyn Rule>> {
        self.values.get(name).cloned()
    }

    #[cfg(test)]
    pub(crate) fn names(&self) -> impl Iterator<Item = &str> {
        self.values.keys().map(String::as_str)
    }
}

#[derive(Clone, Default)]
pub(crate) struct Aliases {
    values: BTreeMap<String, Vec<Expr>>,
}

impl Aliases {
    pub(crate) fn new() -> Self {
        Self::default()
    }

    pub(crate) fn insert(
        &mut self,
        name: impl Into<String>,
        expr: impl AsRef<str>,
    ) -> Result<(), Error> {
        let name = name.into();
        validate_name(&name).map_err(|name| Error::InvalidAliasName { name })?;
        let specs = parse_expression(expr.as_ref())?;
        self.values.insert(name, specs);
        Ok(())
    }

    pub(crate) fn get(&self, name: &str) -> Option<&[Expr]> {
        self.values.get(name).map(Vec::as_slice)
    }

    #[cfg(test)]
    pub(crate) fn names(&self) -> impl Iterator<Item = &str> {
        self.values.keys().map(String::as_str)
    }
}

fn validate_name(name: &str) -> Result<(), String> {
    let valid = !name.is_empty()
        && name
            .chars()
            .all(|ch| ch.is_ascii_alphanumeric() || ch == '_' || ch == '-');

    if valid { Ok(()) } else { Err(name.to_owned()) }
}
