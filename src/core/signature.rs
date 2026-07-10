use std::collections::BTreeSet;

use super::{Error, Params, RawParam, RawParams};

#[derive(Clone, Copy, Debug)]
enum Shape {
    None,
    Text {
        name: &'static str,
        optional: bool,
    },
    List {
        name: &'static str,
    },
    Named {
        names: &'static [&'static str],
        required: &'static [&'static str],
    },
    Pairs {
        name: &'static str,
    },
}

#[derive(Clone, Copy, Debug)]
pub struct Signature {
    shape: Shape,
    fields: bool,
}

impl Signature {
    pub const fn none() -> Self {
        Self {
            shape: Shape::None,
            fields: false,
        }
    }

    pub const fn text(name: &'static str) -> Self {
        Self {
            shape: Shape::Text {
                name,
                optional: false,
            },
            fields: false,
        }
    }

    pub const fn optional_text(name: &'static str) -> Self {
        Self {
            shape: Shape::Text {
                name,
                optional: true,
            },
            fields: false,
        }
    }

    pub const fn list(name: &'static str) -> Self {
        Self {
            shape: Shape::List { name },
            fields: false,
        }
    }

    pub const fn named(names: &'static [&'static str], required: &'static [&'static str]) -> Self {
        Self {
            shape: Shape::Named { names, required },
            fields: false,
        }
    }

    pub const fn pairs(name: &'static str) -> Self {
        Self {
            shape: Shape::Pairs { name },
            fields: false,
        }
    }

    pub const fn with_fields(mut self) -> Self {
        self.fields = true;
        self
    }

    pub(crate) fn requires_fields(self) -> bool {
        self.fields
    }

    pub(crate) fn bind(self, rule: &str, params: &RawParams) -> Result<Params, Error> {
        if !params.positional_values().is_empty() && !params.named_values().is_empty() {
            return Err(invalid(rule, "cannot mix positional and named parameters"));
        }

        match self.shape {
            Shape::None => bind_none(rule, params),
            Shape::Text { name, optional } => bind_text(rule, params, name, optional),
            Shape::List { name } => bind_list(rule, params, name),
            Shape::Named { names, required } => bind_named(rule, params, names, required),
            Shape::Pairs { name } => bind_pairs(rule, params, name),
        }
    }
}

fn bind_none(rule: &str, params: &RawParams) -> Result<Params, Error> {
    if params.is_empty() {
        Ok(Params::new())
    } else {
        Err(invalid(rule, "rule does not accept parameters"))
    }
}

fn bind_text(
    rule: &str,
    params: &RawParams,
    name: &'static str,
    optional: bool,
) -> Result<Params, Error> {
    let value = match (params.positional_values(), params.named_values()) {
        ([], []) if optional => return Ok(Params::new()),
        ([value], []) => value.clone(),
        ([], [(actual, RawParam::Text(value))]) if actual == name => value.clone(),
        ([], [(actual, RawParam::List(_))]) if actual == name => {
            return Err(invalid(rule, format!("parameter '{name}' must be text")));
        }
        ([], named) if named.iter().any(|(actual, _)| actual != name) => {
            return Err(invalid(rule, unknown_names(named, &[name])));
        }
        ([], _) | (_, []) => {
            return Err(invalid(
                rule,
                format!("rule requires exactly one '{name}' parameter"),
            ));
        }
        _ => unreachable!("mixed parameters are rejected before binding"),
    };

    let mut params = Params::new();
    params.insert(name, value);
    Ok(params)
}

fn bind_list(rule: &str, params: &RawParams, name: &'static str) -> Result<Params, Error> {
    let values = match (params.positional_values(), params.named_values()) {
        ([], []) => return Err(invalid(rule, format!("rule requires '{name}' values"))),
        (values, []) => values.to_vec(),
        ([], [(actual, RawParam::List(values))]) if actual == name => values.clone(),
        ([], [(actual, RawParam::Text(value))]) if actual == name => vec![value.clone()],
        ([], named) if named.iter().any(|(actual, _)| actual != name) => {
            return Err(invalid(rule, unknown_names(named, &[name])));
        }
        ([], _) => {
            return Err(invalid(
                rule,
                format!("rule accepts one '{name}' list parameter"),
            ));
        }
        _ => unreachable!("mixed parameters are rejected before binding"),
    };

    if values.is_empty() {
        return Err(invalid(rule, format!("parameter '{name}' cannot be empty")));
    }

    let mut params = Params::new();
    params.insert_list(name, values);
    Ok(params)
}

fn bind_named(
    rule: &str,
    params: &RawParams,
    names: &'static [&'static str],
    required: &'static [&'static str],
) -> Result<Params, Error> {
    if !params.positional_values().is_empty() {
        return Err(invalid(rule, "rule requires named parameters"));
    }
    if params.named_values().is_empty() {
        return Err(invalid(rule, "rule requires at least one parameter"));
    }

    let mut seen = BTreeSet::new();
    let mut bound = Params::new();
    for (name, value) in params.named_values() {
        if !names.contains(&name.as_str()) {
            return Err(invalid(rule, format!("unknown parameter '{name}'")));
        }
        if !seen.insert(name.as_str()) {
            return Err(invalid(rule, format!("duplicate parameter '{name}'")));
        }
        let RawParam::Text(value) = value else {
            return Err(invalid(rule, format!("parameter '{name}' must be text")));
        };
        bound.insert(name, value);
    }

    for name in required {
        if !seen.contains(name) {
            return Err(invalid(rule, format!("missing parameter '{name}'")));
        }
    }

    Ok(bound)
}

fn bind_pairs(rule: &str, params: &RawParams, name: &'static str) -> Result<Params, Error> {
    if !params.positional_values().is_empty() || params.named_values().is_empty() {
        return Err(invalid(rule, "rule requires one or more field=value pairs"));
    }

    let mut seen = BTreeSet::new();
    let mut pairs = Vec::new();
    for (field, value) in params.named_values() {
        if !seen.insert(field.as_str()) {
            return Err(invalid(rule, format!("duplicate field '{field}'")));
        }
        let RawParam::Text(value) = value else {
            return Err(invalid(rule, format!("field '{field}' value must be text")));
        };
        pairs.push((field.clone(), value.clone()));
    }

    let mut params = Params::new();
    params.insert_pairs(name, pairs);
    Ok(params)
}

fn unknown_names(named: &[(String, RawParam)], allowed: &[&str]) -> String {
    let names = named
        .iter()
        .filter(|(name, _)| !allowed.contains(&name.as_str()))
        .map(|(name, _)| name.as_str())
        .collect::<Vec<_>>()
        .join(", ");
    format!("unknown parameter '{names}'")
}

fn invalid(rule: &str, reason: impl Into<String>) -> Error {
    Error::InvalidRuleExpression {
        expression: rule.to_owned(),
        reason: reason.into(),
    }
}
