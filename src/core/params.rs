use std::collections::BTreeMap;
use std::fmt;

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Param {
    Text(String),
    List(Vec<String>),
    Pairs(Vec<(String, String)>),
}

impl fmt::Display for Param {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Text(value) => f.write_str(value),
            Self::List(values) => f.write_str(&values.join(",")),
            Self::Pairs(values) => {
                for (index, (name, value)) in values.iter().enumerate() {
                    if index > 0 {
                        f.write_str(",")?;
                    }
                    write!(f, "{name}={value}")?;
                }
                Ok(())
            }
        }
    }
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct Params {
    values: BTreeMap<String, Param>,
}

impl Params {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn insert(&mut self, name: impl Into<String>, value: impl Into<String>) {
        self.values.insert(name.into(), Param::Text(value.into()));
    }

    pub fn insert_list(&mut self, name: impl Into<String>, values: Vec<String>) {
        self.values.insert(name.into(), Param::List(values));
    }

    pub fn insert_pairs(&mut self, name: impl Into<String>, values: Vec<(String, String)>) {
        self.values.insert(name.into(), Param::Pairs(values));
    }

    pub fn text(&self, name: &str) -> Option<&str> {
        match self.values.get(name) {
            Some(Param::Text(value)) => Some(value),
            Some(Param::List(_) | Param::Pairs(_)) | None => None,
        }
    }

    pub fn list(&self, name: &str) -> Option<&[String]> {
        match self.values.get(name) {
            Some(Param::List(values)) => Some(values),
            Some(Param::Text(_) | Param::Pairs(_)) | None => None,
        }
    }

    pub fn pairs(&self, name: &str) -> Option<&[(String, String)]> {
        match self.values.get(name) {
            Some(Param::Pairs(values)) => Some(values),
            Some(Param::Text(_) | Param::List(_)) | None => None,
        }
    }

    pub fn is_empty(&self) -> bool {
        self.values.is_empty()
    }

    pub fn iter(&self) -> impl Iterator<Item = (&str, &Param)> {
        self.values
            .iter()
            .map(|(name, value)| (name.as_str(), value))
    }
}
