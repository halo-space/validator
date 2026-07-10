#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub(crate) enum RawParam {
    Text(String),
    List(Vec<String>),
}

#[derive(Clone, Debug, Default, Eq, Ord, PartialEq, PartialOrd)]
pub struct RawParams {
    positional: Vec<String>,
    named: Vec<(String, RawParam)>,
}

impl RawParams {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn positional(&mut self, value: impl Into<String>) {
        self.positional.push(value.into());
    }

    pub fn named(&mut self, name: impl Into<String>, value: impl Into<String>) {
        self.named.push((name.into(), RawParam::Text(value.into())));
    }

    pub fn named_list(&mut self, name: impl Into<String>, values: Vec<String>) {
        self.named.push((name.into(), RawParam::List(values)));
    }

    pub(crate) fn positional_values(&self) -> &[String] {
        &self.positional
    }

    pub(crate) fn named_values(&self) -> &[(String, RawParam)] {
        &self.named
    }

    pub(crate) fn is_empty(&self) -> bool {
        self.positional.is_empty() && self.named.is_empty()
    }
}
