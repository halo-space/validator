use std::collections::BTreeSet;

use syn::meta::ParseNestedMeta;
use syn::{
    Expr, ExprLit, ExprUnary, Lit, LitStr, Token, UnOp, bracketed, parenthesized,
};

use super::{DiveAttr, ItemPath, ParamAttr, RuleAttr, canonical, parse_field_path};

const CONDITIONAL_PAIR_RULES: &[&str] = &[
    "required_if",
    "required_unless",
    "skip_unless",
    "excluded_if",
    "excluded_unless",
];

const CONDITIONAL_FIELD_LIST_RULES: &[&str] = &[
    "required_with",
    "required_with_all",
    "required_without",
    "required_without_all",
    "excluded_with",
    "excluded_with_all",
    "excluded_without",
    "excluded_without_all",
];

pub(super) fn rules(attrs: &[syn::Attribute]) -> syn::Result<Vec<RuleAttr>> {
    let mut rules = Vec::new();

    for attr in attrs {
        if !attr.path().is_ident("validate") {
            continue;
        }

        attr.parse_nested_meta(|meta| rule_meta(meta, &mut rules))?;
    }

    Ok(rules)
}

fn rule_meta(meta: ParseNestedMeta<'_>, rules: &mut Vec<RuleAttr>) -> syn::Result<()> {
    if meta.path.is_ident("dive") {
        rules.push(RuleAttr::Dive(dive_rules(meta)?));
        return Ok(());
    }

    if meta.path.is_ident("keys") || meta.path.is_ident("values") {
        return Err(meta.error("keys(...) and values(...) are only supported by map dive"));
    }

    if meta.path.is_ident("required") {
        rules.push(rule("required", Vec::new()));
        return Ok(());
    }
    if meta.path.is_ident("omitempty") {
        rules.push(RuleAttr::OmitEmpty);
        return Ok(());
    }
    if meta.path.is_ident("nested") {
        rules.push(RuleAttr::Nested);
        return Ok(());
    }
    if meta.path.is_ident("alias") {
        let value = meta.value()?;
        let alias: LitStr = value.parse()?;
        rules.push(RuleAttr::Alias(alias.value()));
        return Ok(());
    }

    if let Some(name) = meta.path.get_ident().map(canonical)
        && (name.ends_with("_field") || matches!(name.as_str(), "fieldcontains" | "fieldexcludes"))
    {
        let value = meta.value()?;
        let target: LitStr = value.parse()?;
        rules.push(RuleAttr::FieldRule {
            name,
            params: vec![ParamAttr::Named("compare".to_owned(), target.value())],
        });
        return Ok(());
    }

    for name in CONDITIONAL_PAIR_RULES {
        if meta.path.is_ident(*name) {
            rules.push(RuleAttr::FieldRule {
                name: (*name).to_owned(),
                params: conditional_pairs(meta, name)?,
            });
            return Ok(());
        }
    }

    for name in CONDITIONAL_FIELD_LIST_RULES {
        if meta.path.is_ident(*name) {
            rules.push(RuleAttr::FieldRule {
                name: (*name).to_owned(),
                params: vec![ParamAttr::List(
                    "fields".to_owned(),
                    field_list(meta, name)?,
                )],
            });
            return Ok(());
        }
    }

    if meta.path.is_ident("unique") && meta.input.peek(Token![=]) {
        rules.push(RuleAttr::UniqueFields {
            paths: unique_paths(meta)?,
        });
        return Ok(());
    }
    if meta.path.is_ident("unique") && meta.input.peek(syn::token::Paren) {
        return Err(meta
            .error("unique field syntax is `unique = \"field\"` or `unique = [\"field\", ...]`"));
    }

    let Some(name) = meta.path.get_ident().map(canonical) else {
        return Err(meta.error("validate rule must be a single identifier"));
    };
    rules.push(rule(name, custom_params(meta)?));
    Ok(())
}

fn unique_paths(meta: ParseNestedMeta<'_>) -> syn::Result<Vec<ItemPath>> {
    let input = meta.value()?;
    let paths = if input.peek(LitStr) {
        vec![unique_path(input.parse()?)?]
    } else if input.peek(syn::token::Bracket) {
        let content;
        bracketed!(content in input);
        let mut paths = Vec::new();
        while !content.is_empty() {
            paths.push(unique_path(content.parse()?)?);
            if content.peek(Token![,]) {
                content.parse::<Token![,]>()?;
            } else if !content.is_empty() {
                return Err(content.error("expected ',' between unique field paths"));
            }
        }
        paths
    } else {
        return Err(input.error("unique expects a field string or an array of field strings"));
    };

    if paths.is_empty() {
        return Err(syn::Error::new(
            proc_macro2::Span::call_site(),
            "unique field list cannot be empty",
        ));
    }

    let mut seen = BTreeSet::new();
    if let Some(duplicate) = paths.iter().find(|path| !seen.insert(path.name.as_str())) {
        return Err(syn::Error::new(
            proc_macro2::Span::call_site(),
            format!("unique field path '{}' is repeated", duplicate.name),
        ));
    }

    Ok(paths)
}

fn unique_path(field: LitStr) -> syn::Result<ItemPath> {
    let name = field.value();
    let segments = parse_field_path("unique", &name)?;
    Ok(ItemPath { name, segments })
}

fn custom_params(meta: ParseNestedMeta<'_>) -> syn::Result<Vec<ParamAttr>> {
    if meta.input.peek(Token![=]) {
        return Ok(vec![ParamAttr::Positional(param_value(meta.value()?)?)]);
    }
    if !meta.input.peek(syn::token::Paren) {
        return Ok(Vec::new());
    }

    let content;
    parenthesized!(content in meta.input);
    let mut params = Vec::new();

    while !content.is_empty() {
        if content.peek(syn::Ident) && content.peek2(Token![=]) {
            let name: syn::Ident = content.parse()?;
            content.parse::<Token![=]>()?;
            params.push(ParamAttr::Named(
                canonical(&name),
                param_value(&content)?,
            ));
        } else {
            params.push(ParamAttr::Positional(param_value(&content)?));
        }

        if content.peek(Token![,]) {
            content.parse::<Token![,]>()?;
        } else if !content.is_empty() {
            return Err(content.error("expected ',' between validate parameters"));
        }
    }

    Ok(params)
}

fn dive_rules(meta: ParseNestedMeta<'_>) -> syn::Result<DiveAttr> {
    let mut rules = Vec::new();
    let mut keys = None;
    let mut values = None;

    meta.parse_nested_meta(|nested| {
        if nested.path.is_ident("keys") {
            if keys.is_some() {
                return Err(nested.error("duplicate keys(...) in dive"));
            }
            keys = Some(dive_section(nested)?);
            return Ok(());
        }

        if nested.path.is_ident("values") {
            if values.is_some() {
                return Err(nested.error("duplicate values(...) in dive"));
            }
            values = Some(dive_section(nested)?);
            return Ok(());
        }

        rule_meta(nested, &mut rules)
    })?;

    match (keys, values) {
        (Some(keys), Some(values)) => {
            if !rules.is_empty() {
                return Err(meta.error("map dive cannot mix keys/values with bare element rules"));
            }
            Ok(DiveAttr::Map { keys, values })
        }
        (Some(_), None) | (None, Some(_)) => {
            Err(meta.error("map dive requires both keys(...) and values(...)"))
        }
        (None, None) => {
            if rules.is_empty() {
                return Err(meta.error("dive requires at least one rule"));
            }
            Ok(DiveAttr::Values(rules))
        }
    }
}

fn dive_section(meta: ParseNestedMeta<'_>) -> syn::Result<Vec<RuleAttr>> {
    let mut rules = Vec::new();
    meta.parse_nested_meta(|nested| rule_meta(nested, &mut rules))?;

    if rules.is_empty() {
        return Err(meta.error("map dive section requires at least one rule"));
    }

    Ok(rules)
}

fn conditional_pairs(meta: ParseNestedMeta<'_>, rule: &str) -> syn::Result<Vec<ParamAttr>> {
    let mut params = Vec::new();
    let mut seen = BTreeSet::new();

    meta.parse_nested_meta(|nested| {
        let Some(field) = nested.path.get_ident().map(canonical) else {
            return Err(nested.error("conditional rule field must be a field identifier"));
        };

        if !seen.insert(field.clone()) {
            return Err(nested.error(format!("duplicate field '{field}' in {rule}")));
        }

        params.push(ParamAttr::Named(field, param_value(nested.value()?)?));
        Ok(())
    })?;

    if params.is_empty() {
        return Err(meta.error(format!("{rule} requires at least one field condition")));
    }

    Ok(params)
}

fn field_list(meta: ParseNestedMeta<'_>, rule: &str) -> syn::Result<Vec<String>> {
    let content;
    parenthesized!(content in meta.input);

    let mut fields = Vec::new();
    while !content.is_empty() {
        let field: LitStr = content.parse()?;
        fields.push(field.value());

        if content.peek(Token![,]) {
            content.parse::<Token![,]>()?;
        }
    }

    if fields.is_empty() {
        return Err(meta.error(format!("{rule} requires at least one field")));
    }

    Ok(fields)
}

fn rule(name: impl Into<String>, params: Vec<ParamAttr>) -> RuleAttr {
    RuleAttr::Rule {
        name: name.into(),
        params,
    }
}

fn param_value(input: syn::parse::ParseStream<'_>) -> syn::Result<String> {
    let expr: Expr = input.parse()?;
    match expr {
        Expr::Lit(ExprLit { lit, .. }) => literal(lit),
        Expr::Unary(ExprUnary {
            op: UnOp::Neg(_),
            expr,
            ..
        }) => match *expr {
            Expr::Lit(ExprLit {
                lit: Lit::Int(value),
                ..
            }) => Ok(format!("-{}", value.base10_digits())),
            Expr::Lit(ExprLit {
                lit: Lit::Float(value),
                ..
            }) => Ok(format!("-{}", value.base10_digits())),
            expr => Err(syn::Error::new_spanned(
                expr,
                "negative validate parameter must be an integer or float literal",
            )),
        },
        Expr::Path(path) if path.path.is_ident("true") => Ok("true".to_owned()),
        Expr::Path(path) if path.path.is_ident("false") => Ok("false".to_owned()),
        expr => Err(syn::Error::new_spanned(
            expr,
            "validate parameter must be an integer, float, string, or bool literal",
        )),
    }
}

fn literal(lit: Lit) -> syn::Result<String> {
    match lit {
        Lit::Int(value) => Ok(value.base10_digits().to_owned()),
        Lit::Float(value) => Ok(value.base10_digits().to_owned()),
        Lit::Str(value) => Ok(value.value()),
        Lit::Bool(value) => Ok(value.value.to_string()),
        _ => Err(syn::Error::new_spanned(
            lit,
            "validate parameter must be an integer, float, string, or bool literal",
        )),
    }
}
