use std::collections::{BTreeMap, BTreeSet};

use super::ident::{canonical, member_name};
use super::model::{ParamAttr, RuleAttr};

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

fn exposes_value_access(rules: &[RuleAttr]) -> bool {
    let has_nested = rules.iter().any(|rule| matches!(rule, RuleAttr::Nested));

    rules.iter().any(|rule| match rule {
        RuleAttr::Rule { name, .. } => !(has_nested && name == "required"),
        RuleAttr::Alias(_) => true,
        RuleAttr::FieldRule { .. } => true,
        RuleAttr::UniqueFields { .. } => false,
        RuleAttr::OmitEmpty => !has_nested,
        RuleAttr::Nested | RuleAttr::Dive(_) => false,
    })
}

pub(super) fn collect_access(
    rules: &[RuleAttr],
    current: &syn::Ident,
    field_members: &BTreeMap<String, syn::Ident>,
    access_fields: &mut BTreeSet<String>,
    access_paths: &mut BTreeMap<String, Vec<syn::Ident>>,
) -> syn::Result<()> {
    if exposes_value_access(rules) {
        access_fields.insert(canonical(current));
    }

    for rule in rules {
        if let RuleAttr::FieldRule { name, params } = rule {
            for target in field_targets(name, params) {
                if supports_nested_target(name) {
                    let mut segments = parse_field_path(name, target)?;
                    let first_name = canonical(
                        segments
                            .first()
                            .expect("validated field path must contain a segment"),
                    );
                    let Some(first) = field_members.get(&first_name) else {
                        return Err(syn::Error::new(
                            proc_macro2::Span::call_site(),
                            format!(
                                "validate rule '{name}' references unknown field '{first_name}' in path '{target}'"
                            ),
                        ));
                    };
                    segments[0] = first.clone();

                    if segments.len() == 1 {
                        access_fields.insert(first_name);
                    } else {
                        access_paths.entry(target.to_owned()).or_insert(segments);
                    }
                } else if field_members.contains_key(target) {
                    access_fields.insert(target.to_owned());
                } else {
                    return Err(syn::Error::new(
                        proc_macro2::Span::call_site(),
                        format!("validate rule '{name}' references unknown field '{target}'"),
                    ));
                }
            }
        }
    }

    Ok(())
}

fn supports_nested_target(rule: &str) -> bool {
    !CONDITIONAL_PAIR_RULES.contains(&rule) && !CONDITIONAL_FIELD_LIST_RULES.contains(&rule)
}

pub(super) fn parse_field_path(rule: &str, path: &str) -> syn::Result<Vec<syn::Ident>> {
    if path.is_empty() {
        return Err(invalid_field_path(rule, path));
    }

    path.split('.')
        .map(|segment| {
            if segment.is_empty() {
                return Err(invalid_field_path(rule, path));
            }

            let ident = member_name(segment).ok_or_else(|| invalid_field_path(rule, path))?;
            if canonical(&ident) != segment {
                return Err(invalid_field_path(rule, path));
            }
            Ok(ident)
        })
        .collect()
}

fn invalid_field_path(rule: &str, path: &str) -> syn::Error {
    syn::Error::new(
        proc_macro2::Span::call_site(),
        format!(
            "validate rule '{rule}' has invalid field path '{path}'; expected dot-separated Rust field identifiers"
        ),
    )
}

fn field_targets<'a>(rule: &str, params: &'a [ParamAttr]) -> Vec<&'a str> {
    match rule {
        "required_with"
        | "required_with_all"
        | "required_without"
        | "required_without_all"
        | "excluded_with"
        | "excluded_with_all"
        | "excluded_without"
        | "excluded_without_all" => params
            .iter()
            .find_map(|param| match param {
                ParamAttr::List(name, values) if name == "fields" => Some(values),
                _ => None,
            })
            .into_iter()
            .flatten()
            .map(String::as_str)
            .collect(),
        "required_if" | "required_unless" | "skip_unless" | "excluded_if" | "excluded_unless" => {
            params
                .iter()
                .filter_map(|param| match param {
                    ParamAttr::Named(field, _) => Some(field.as_str()),
                    _ => None,
                })
                .collect::<Vec<_>>()
        }
        _ => params
            .iter()
            .find_map(|param| match param {
                ParamAttr::Named(name, compare) if name == "compare" => Some(compare.as_str()),
                _ => None,
            })
            .map(|compare| vec![compare])
            .unwrap_or_default(),
    }
}
