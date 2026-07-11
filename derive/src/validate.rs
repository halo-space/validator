use super::model::{DiveAttr, RuleAttr};

pub(super) fn reject_field_rules_inside_dive(rules: &[RuleAttr]) -> syn::Result<()> {
    for rule in rules {
        match rule {
            RuleAttr::Dive(DiveAttr::Values(rules)) => field_rules(rules)?,
            RuleAttr::Dive(DiveAttr::Map { keys, values }) => {
                field_rules(keys)?;
                field_rules(values)?;
            }
            RuleAttr::Rule { .. }
            | RuleAttr::Alias(_)
            | RuleAttr::FieldRule { .. }
            | RuleAttr::UniqueFields { .. }
            | RuleAttr::OmitEmpty
            | RuleAttr::Nested => {}
        }
    }

    Ok(())
}

fn field_rules(rules: &[RuleAttr]) -> syn::Result<()> {
    for rule in rules {
        match rule {
            RuleAttr::FieldRule { name, .. } => {
                return Err(syn::Error::new(
                    proc_macro2::Span::call_site(),
                    format!("validate rule '{name}' is not supported inside dive"),
                ));
            }
            RuleAttr::UniqueFields { .. } => {
                return Err(syn::Error::new(
                    proc_macro2::Span::call_site(),
                    "parameterized unique is not supported inside dive",
                ));
            }
            RuleAttr::Dive(DiveAttr::Values(rules)) => field_rules(rules)?,
            RuleAttr::Dive(DiveAttr::Map { keys, values }) => {
                field_rules(keys)?;
                field_rules(values)?;
            }
            RuleAttr::Rule { .. } | RuleAttr::Alias(_) | RuleAttr::OmitEmpty | RuleAttr::Nested => {
            }
        }
    }

    Ok(())
}
