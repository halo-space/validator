use std::collections::BTreeMap;
use std::sync::Arc;

use super::spec::{RuleGroup, parse_rule_expression};
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

    pub(crate) fn contains(&self, name: &str) -> bool {
        self.values.contains_key(name)
    }
}

#[derive(Clone, Default)]
pub(crate) struct Aliases {
    values: BTreeMap<String, Vec<RuleGroup>>,
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
        let specs = parse_rule_expression(expr.as_ref())?;
        self.values.insert(name, specs);
        Ok(())
    }

    pub(crate) fn get(&self, name: &str) -> Option<&[RuleGroup]> {
        self.values.get(name).map(Vec::as_slice)
    }

    pub(crate) fn contains(&self, name: &str) -> bool {
        self.values.contains_key(name)
    }
}

fn validate_name(name: &str) -> Result<(), String> {
    let valid = !name.is_empty()
        && name
            .chars()
            .all(|ch| ch.is_ascii_alphanumeric() || ch == '_' || ch == '-');

    if valid { Ok(()) } else { Err(name.to_owned()) }
}
