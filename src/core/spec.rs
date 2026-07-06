use super::{Args, Error};

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct RuleSpec {
    name: String,
    args: Args,
}

impl RuleSpec {
    pub(crate) fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            args: Args::new(),
        }
    }

    pub(crate) fn arg(mut self, name: impl Into<String>, value: impl Into<String>) -> Self {
        self.args.insert(name, value);
        self
    }

    pub(crate) fn name(&self) -> &str {
        &self.name
    }

    pub(crate) fn args(&self) -> &Args {
        &self.args
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) enum RuleGroup {
    Single(RuleSpec),
    Any(Vec<RuleSpec>),
}

impl RuleGroup {
    pub(crate) fn single(&self) -> Option<&RuleSpec> {
        match self {
            Self::Single(spec) => Some(spec),
            Self::Any(_) => None,
        }
    }

    pub(crate) fn alternatives(&self) -> Option<&[RuleSpec]> {
        match self {
            Self::Single(_) => None,
            Self::Any(specs) => Some(specs),
        }
    }
}

pub(crate) fn parse_rule_expression(expr: &str) -> Result<Vec<RuleGroup>, Error> {
    let mut groups = Vec::new();

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
            [_] => groups.push(RuleGroup::Single(
                alternatives
                    .into_iter()
                    .next()
                    .expect("single alternative must exist"),
            )),
            _ => groups.push(RuleGroup::Any(alternatives)),
        }
    }

    Ok(groups)
}

fn parse_rule(item: &str) -> Result<RuleSpec, Error> {
    if let Some((name, rest)) = item.split_once('(') {
        let name = name.trim();
        let args = rest
            .strip_suffix(')')
            .ok_or_else(|| invalid_alias(item, "missing closing ')'"))?;
        let mut spec = RuleSpec::new(name);
        let mut positional = Vec::new();

        for pair in split_top_level(args, ',') {
            let pair = pair.trim();
            if pair.is_empty() {
                continue;
            }
            if let Some((key, value)) = pair.split_once('=') {
                spec = spec.arg(key.trim(), trim_quotes(value.trim()));
            } else {
                positional.push(trim_quotes(pair).to_owned());
            }
        }

        if !positional.is_empty() {
            if name != "oneof" {
                return Err(invalid_alias(item, "expected key=value argument"));
            }
            spec = spec.arg("values", positional.join(","));
        }

        return Ok(spec);
    }

    if let Some((name, value)) = item.split_once('=') {
        let name = name.trim();
        let arg = match name {
            "min" => "min",
            "max" => "max",
            "regex" => "pattern",
            "oneof" => "values",
            _ => "value",
        };
        return Ok(RuleSpec::new(name).arg(arg, trim_quotes(value.trim())));
    }

    Ok(RuleSpec::new(item.trim()))
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

fn invalid_alias(item: &str, reason: &str) -> Error {
    Error::InvalidAlias {
        name: item.to_owned(),
        reason: reason.to_owned(),
    }
}

#[cfg(test)]
mod tests {
    use super::{RuleGroup, parse_rule_expression};

    #[test]
    fn parses_rule_expression() {
        let groups = parse_rule_expression("required,length(min=3,max=20)").unwrap();

        assert_eq!(groups.len(), 2);
        let required = groups[0].single().unwrap();
        let length = groups[1].single().unwrap();

        assert_eq!(required.name(), "required");
        assert_eq!(length.name(), "length");
        assert_eq!(length.args().get("min"), Some("3"));
        assert_eq!(length.args().get("max"), Some("20"));
    }

    #[test]
    fn parses_oneof_with_quoted_values() {
        let groups = parse_rule_expression(r#"oneof("draft","published")"#).unwrap();
        let spec = groups[0].single().unwrap();

        assert_eq!(groups.len(), 1);
        assert_eq!(spec.name(), "oneof");
        assert_eq!(spec.args().get("values"), Some("draft,published"));
    }

    #[test]
    fn parses_rule_alternatives() {
        let groups = parse_rule_expression("required,hexcolor|rgb|rgba").unwrap();

        assert_eq!(groups.len(), 2);
        assert!(matches!(groups[0], RuleGroup::Single(_)));

        let alternatives = groups[1].alternatives().unwrap();
        let names = alternatives
            .iter()
            .map(|spec| spec.name())
            .collect::<Vec<_>>();

        assert_eq!(names, vec!["hexcolor", "rgb", "rgba"]);
    }
}
