#![allow(missing_docs)]

use super::RawParams;

mod parser;

#[cfg(test)]
mod tests;

pub(crate) use self::parser::parse_expression;

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
#[doc(hidden)]
pub struct Spec {
    name: String,
    params: RawParams,
}

impl Spec {
    pub(crate) fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            params: RawParams::new(),
        }
    }

    #[doc(hidden)]
    pub fn with_params(name: impl Into<String>, params: RawParams) -> Self {
        Self {
            name: name.into(),
            params,
        }
    }

    pub(crate) fn positional(mut self, value: impl Into<String>) -> Self {
        self.params.positional(value);
        self
    }

    pub(crate) fn named(mut self, name: impl Into<String>, value: impl Into<String>) -> Self {
        self.params.named(name, value);
        self
    }

    pub(crate) fn named_list(mut self, name: impl Into<String>, values: Vec<String>) -> Self {
        self.params.named_list(name, values);
        self
    }

    pub(crate) fn name(&self) -> &str {
        &self.name
    }

    pub(crate) fn params(&self) -> &RawParams {
        &self.params
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) enum Expr {
    Single(Spec),
    Any(Vec<Spec>),
}

impl Expr {
    pub(crate) fn single(&self) -> Option<&Spec> {
        match self {
            Self::Single(spec) => Some(spec),
            Self::Any(_) => None,
        }
    }

    pub(crate) fn alternatives(&self) -> Option<&[Spec]> {
        match self {
            Self::Single(_) => None,
            Self::Any(specs) => Some(specs),
        }
    }
}
