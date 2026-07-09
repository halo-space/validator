use super::{Error, Params};

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct Spec {
    name: String,
    params: Params,
}

impl Spec {
    pub(crate) fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            params: Params::new(),
        }
    }

    pub(crate) fn param(mut self, name: impl Into<String>, value: impl Into<String>) -> Self {
        self.params.insert(name, value);
        self
    }

    pub(crate) fn name(&self) -> &str {
        &self.name
    }

    pub(crate) fn params(&self) -> &Params {
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

pub(crate) fn parse_expression(expr: &str) -> Result<Vec<Expr>, Error> {
    let mut exprs = Vec::new();

    for item in split_top_level(expr, ',') {
        let item = item.trim();
        if item.is_empty() {
            continue;
        }

        let alternatives = split_top_level(item, '|')
            .into_iter()
            .map(str::trim)
            .filter(|item| !item.is_empty())
            .map(parse_rule)
            .collect::<Result<Vec<_>, _>>()?;

        match alternatives.as_slice() {
            [] => {}
            [_] => exprs.push(Expr::Single(
                alternatives
                    .into_iter()
                    .next()
                    .expect("single alternative must exist"),
            )),
            _ => exprs.push(Expr::Any(alternatives)),
        }
    }

    Ok(exprs)
}

fn parse_rule(item: &str) -> Result<Spec, Error> {
    if let Some((name, rest)) = item.split_once('(') {
        let name = name.trim();
        let params = rest
            .strip_suffix(')')
            .ok_or_else(|| invalid_rule_expression(item, "missing closing ')'"))?;
        let mut spec = Spec::new(name);
        let mut positional = Vec::new();

        for pair in split_top_level(params, ',') {
            let pair = pair.trim();
            if pair.is_empty() {
                continue;
            }
            if let Some((key, value)) = pair.split_once('=') {
                spec = spec.param(key.trim(), trim_quotes(value.trim()));
            } else {
                positional.push(trim_quotes(pair).to_owned());
            }
        }

        if !positional.is_empty() {
            if !matches!(name, "oneof" | "oneofci" | "noneof" | "noneofci") {
                return Err(invalid_rule_expression(
                    item,
                    "expected key=value parameter",
                ));
            }
            spec = spec.param("values", positional.join(","));
        }

        return Ok(spec);
    }

    if let Some((name, value)) = item.split_once('=') {
        let name = name.trim();
        let param = match name {
            "min" => "min",
            "max" => "max",
            "regex" => "pattern",
            "oneof" | "oneofci" | "noneof" | "noneofci" => "values",
            _ => "value",
        };
        return Ok(Spec::new(name).param(param, trim_quotes(value.trim())));
    }

    Ok(Spec::new(item.trim()))
}

fn split_top_level(input: &str, separator: char) -> Vec<&str> {
    let mut parts = Vec::new();
    let mut start = 0;
    let mut depth = 0;
    let mut quote = None;
    let mut escaped = false;

    for (index, ch) in input.char_indices() {
        if let Some(current_quote) = quote {
            if escaped {
                escaped = false;
            } else if ch == '\\' {
                escaped = true;
            } else if ch == current_quote {
                quote = None;
            }
            continue;
        }

        match ch {
            '"' | '\'' => quote = Some(ch),
            '(' => depth += 1,
            ')' if depth > 0 => depth -= 1,
            ch if ch == separator && depth == 0 => {
                parts.push(&input[start..index]);
                start = index + ch.len_utf8();
            }
            _ => {}
        }
    }

    parts.push(&input[start..]);
    parts
}

fn trim_quotes(value: &str) -> &str {
    if let Some(value) = value
        .strip_prefix('"')
        .and_then(|value| value.strip_suffix('"'))
    {
        return value;
    }

    value
        .strip_prefix('\'')
        .and_then(|value| value.strip_suffix('\''))
        .unwrap_or(value)
}

fn invalid_rule_expression(item: &str, reason: &str) -> Error {
    Error::InvalidRuleExpression {
        expression: item.to_owned(),
        reason: reason.to_owned(),
    }
}

#[cfg(test)]
mod tests {
    use super::{Expr, parse_expression};

    #[test]
    fn parses_rule_expression() {
        let exprs = parse_expression("required,length(min=3,max=20)").unwrap();

        assert_eq!(exprs.len(), 2);
        let required = exprs[0].single().unwrap();
        let length = exprs[1].single().unwrap();

        assert_eq!(required.name(), "required");
        assert_eq!(length.name(), "length");
        assert_eq!(length.params().get("min"), Some("3"));
        assert_eq!(length.params().get("max"), Some("20"));
    }

    #[test]
    fn parses_oneof_with_quoted_values() {
        let exprs = parse_expression(r#"oneof("draft","published")"#).unwrap();
        let spec = exprs[0].single().unwrap();

        assert_eq!(exprs.len(), 1);
        assert_eq!(spec.name(), "oneof");
        assert_eq!(spec.params().get("values"), Some("draft,published"));
    }

    #[test]
    fn parses_rule_alternatives() {
        let exprs = parse_expression("required,hexcolor|rgb|rgba").unwrap();

        assert_eq!(exprs.len(), 2);
        assert!(matches!(exprs[0], Expr::Single(_)));

        let alternatives = exprs[1].alternatives().unwrap();
        let names = alternatives
            .iter()
            .map(|spec| spec.name())
            .collect::<Vec<_>>();

        assert_eq!(names, vec!["hexcolor", "rgb", "rgba"]);
    }
}
