use super::{Error, RawParams};

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

pub(crate) fn parse_expression(expr: &str) -> Result<Vec<Expr>, Error> {
    let mut exprs = Vec::new();

    for item in
        split_top_level(expr, ',').map_err(|reason| invalid_rule_expression(expr, reason))?
    {
        let item = item.trim();
        if item.is_empty() {
            return Err(invalid_rule_expression(expr, "rule cannot be empty"));
        }

        let alternatives = split_top_level(item, '|')
            .map_err(|reason| invalid_rule_expression(item, reason))?
            .into_iter()
            .map(str::trim)
            .map(|alternative| {
                if alternative.is_empty() {
                    Err(invalid_rule_expression(item, "alternative cannot be empty"))
                } else {
                    parse_rule(alternative)
                }
            })
            .collect::<Result<Vec<_>, _>>()?;

        match alternatives.as_slice() {
            [] => return Err(invalid_rule_expression(item, "rule cannot be empty")),
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
        if name.is_empty() {
            return Err(invalid_rule_expression(item, "rule name cannot be empty"));
        }
        let params = rest
            .strip_suffix(')')
            .ok_or_else(|| invalid_rule_expression(item, "missing closing ')'"))?;
        let mut spec = Spec::new(name);

        if params.trim().is_empty() {
            return Ok(spec);
        }

        for pair in
            split_top_level(params, ',').map_err(|reason| invalid_rule_expression(item, reason))?
        {
            let pair = pair.trim();
            if pair.is_empty() {
                return Err(invalid_rule_expression(item, "parameter cannot be empty"));
            }
            if let Some((key, value)) = split_top_level_once(pair, '=') {
                let key = key.trim();
                if key.is_empty() {
                    return Err(invalid_rule_expression(
                        item,
                        "parameter name cannot be empty",
                    ));
                }
                spec = spec.named(key, decode_param(value, item)?);
            } else {
                spec = spec.positional(decode_param(pair, item)?);
            }
        }

        return Ok(spec);
    }

    if let Some((name, value)) = split_top_level_once(item, '=') {
        let name = name.trim();
        if name.is_empty() {
            return Err(invalid_rule_expression(item, "rule name cannot be empty"));
        }
        return Ok(Spec::new(name).positional(decode_param(value, item)?));
    }

    let name = item.trim();
    if name.is_empty() {
        return Err(invalid_rule_expression(item, "rule name cannot be empty"));
    }
    Ok(Spec::new(name))
}

fn split_top_level(input: &str, separator: char) -> Result<Vec<&str>, &'static str> {
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
            ')' => return Err("unexpected closing ')'"),
            ch if ch == separator && depth == 0 => {
                parts.push(&input[start..index]);
                start = index + ch.len_utf8();
            }
            _ => {}
        }
    }

    if escaped {
        return Err("quoted value has a dangling escape");
    }
    if quote.is_some() {
        return Err("quoted value is not closed");
    }
    if depth != 0 {
        return Err("missing closing ')'");
    }

    parts.push(&input[start..]);
    Ok(parts)
}

fn split_top_level_once(input: &str, separator: char) -> Option<(&str, &str)> {
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
                let value = index + ch.len_utf8();
                return Some((&input[..index], &input[value..]));
            }
            _ => {}
        }
    }

    None
}

fn decode_param(value: &str, expression: &str) -> Result<String, Error> {
    let value = value.trim();
    if value.is_empty() {
        return Err(invalid_rule_expression(
            expression,
            "parameter value cannot be empty",
        ));
    }

    let Some(quote) = value.chars().next().filter(|ch| matches!(ch, '"' | '\'')) else {
        if value.contains(['"', '\'']) {
            return Err(invalid_rule_expression(
                expression,
                "quotes must wrap the entire parameter value",
            ));
        }
        return Ok(value.to_owned());
    };
    if value.len() < 2 || !value.ends_with(quote) {
        return Err(invalid_rule_expression(
            expression,
            "quoted parameter value is not closed",
        ));
    }

    let inner = &value[quote.len_utf8()..value.len() - quote.len_utf8()];
    let mut decoded = String::with_capacity(inner.len());
    let mut escaped = false;
    for ch in inner.chars() {
        if escaped {
            decoded.push(ch);
            escaped = false;
        } else if ch == '\\' {
            escaped = true;
        } else if ch == quote {
            return Err(invalid_rule_expression(
                expression,
                "quote inside parameter value must be escaped",
            ));
        } else {
            decoded.push(ch);
        }
    }
    if escaped {
        return Err(invalid_rule_expression(
            expression,
            "quoted parameter value has a dangling escape",
        ));
    }

    Ok(decoded)
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
        assert_eq!(length.params().named_values().len(), 2);
    }

    #[test]
    fn parses_oneof_with_quoted_values() {
        let exprs = parse_expression(r#"oneof("draft","published")"#).unwrap();
        let spec = exprs[0].single().unwrap();

        assert_eq!(exprs.len(), 1);
        assert_eq!(spec.name(), "oneof");
        assert_eq!(spec.params().positional_values(), ["draft", "published"]);
    }

    #[test]
    fn parses_conditional_field_lists() {
        let exprs = parse_expression(r#"required_with("email","phone")"#).unwrap();
        let spec = exprs[0].single().unwrap();

        assert_eq!(exprs.len(), 1);
        assert_eq!(spec.name(), "required_with");
        assert_eq!(spec.params().positional_values(), ["email", "phone"]);

        let exprs = parse_expression("required_without=email").unwrap();
        let spec = exprs[0].single().unwrap();

        assert_eq!(exprs.len(), 1);
        assert_eq!(spec.name(), "required_without");
        assert_eq!(spec.params().positional_values(), ["email"]);

        let exprs = parse_expression(r#"excluded_with_all("email","phone")"#).unwrap();
        let spec = exprs[0].single().unwrap();

        assert_eq!(exprs.len(), 1);
        assert_eq!(spec.name(), "excluded_with_all");
        assert_eq!(spec.params().positional_values(), ["email", "phone"]);

        let exprs = parse_expression("required_without_all=email").unwrap();
        let spec = exprs[0].single().unwrap();

        assert_eq!(exprs.len(), 1);
        assert_eq!(spec.name(), "required_without_all");
        assert_eq!(spec.params().positional_values(), ["email"]);
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

    #[test]
    fn rejects_empty_and_unbalanced_expressions() {
        for expression in ["", "email,", "email||url", "oneof(\"value)", "email)"] {
            assert!(parse_expression(expression).is_err(), "{expression}");
        }
    }

    #[test]
    fn decodes_escaped_quoted_parameters() {
        let exprs = parse_expression(r#"oneof("a\"b","c\\d")"#).unwrap();
        let values = exprs[0].single().unwrap().params().positional_values();

        assert_eq!(values, ["a\"b", "c\\d"]);
    }
}
