use super::{Expr, Spec};
use crate::core::Error;

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
    if let Some((name, rest)) = split_top_level_once(item, '(') {
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
        let values = split_params(value).map_err(|reason| invalid_rule_expression(item, reason))?;
        if values.is_empty() {
            return Err(invalid_rule_expression(
                item,
                "parameter value cannot be empty",
            ));
        }
        let mut spec = Spec::new(name);
        for value in values {
            spec = spec.positional(decode_param(value, item)?);
        }
        return Ok(spec);
    }

    let name = item.trim();
    if name.is_empty() {
        return Err(invalid_rule_expression(item, "rule name cannot be empty"));
    }
    Ok(Spec::new(name))
}

fn split_params(input: &str) -> Result<Vec<&str>, &'static str> {
    let mut parts = Vec::new();
    let mut start = None;
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
            '"' | '\'' => {
                quote = Some(ch);
                start.get_or_insert(index);
            }
            ch if ch.is_whitespace() => {
                if let Some(start) = start.take() {
                    parts.push(&input[start..index]);
                }
            }
            _ => {
                start.get_or_insert(index);
            }
        }
    }

    if escaped {
        return Err("quoted value has a dangling escape");
    }
    if quote.is_some() {
        return Err("quoted value is not closed");
    }
    if let Some(start) = start {
        parts.push(&input[start..]);
    }

    Ok(parts)
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
            ch if ch == separator && depth == 0 => {
                let value = index + ch.len_utf8();
                return Some((&input[..index], &input[value..]));
            }
            '(' => depth += 1,
            ')' if depth > 0 => depth -= 1,
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
