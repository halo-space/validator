use std::fmt;

#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
/// A dotted field path that can include collection indices and map keys.
pub struct Namespace(String);

impl Namespace {
    /// Creates a namespace from its rendered path.
    pub fn new(value: impl Into<String>) -> Self {
        Self(value.into())
    }

    /// Returns the rendered namespace path.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl AsRef<str> for Namespace {
    fn as_ref(&self) -> &str {
        self.as_str()
    }
}

impl fmt::Display for Namespace {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}
