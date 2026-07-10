use proc_macro::TokenStream;
use proc_macro_crate::{FoundCrate, crate_name};
use quote::{format_ident, quote};
use std::collections::BTreeSet;
use syn::ext::IdentExt;
use syn::meta::ParseNestedMeta;
use syn::{
    Data, DeriveInput, Expr, ExprLit, ExprUnary, Fields, GenericArgument, Lit, LitStr,
    PathArguments, Token, Type, TypeArray, TypeReference, UnOp, parenthesized, parse_macro_input,
};

#[proc_macro_derive(Validate, attributes(validate))]
pub fn derive_validate(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);

    match expand_validate(input) {
        Ok(tokens) => tokens.into(),
        Err(error) => error.to_compile_error().into(),
    }
}

fn expand_validate(input: DeriveInput) -> syn::Result<proc_macro2::TokenStream> {
    let crate_path = validator_crate_path()?;
    let DeriveInput {
        attrs,
        data,
        generics,
        ident: name,
        ..
    } = input;
    let type_name = canonical(&name);
    let struct_checks = parse_struct_checks(&attrs)?;
    let fields = match data {
        Data::Struct(data) => match data.fields {
            Fields::Named(fields) => fields.named,
            _ => {
                return Err(syn::Error::new_spanned(
                    name,
                    "Validate can only be derived for structs with named fields",
                ));
            }
        },
        _ => {
            return Err(syn::Error::new_spanned(
                name,
                "Validate can only be derived for structs",
            ));
        }
    };
    let (impl_generics, ty_generics, where_clause) = generics.split_for_impl();
    let fields = fields.into_iter().collect::<Vec<_>>();
    let field_names = fields
        .iter()
        .filter_map(|field| field.ident.as_ref())
        .map(canonical)
        .collect::<BTreeSet<_>>();
    let mut access_fields = BTreeSet::new();

    let mut checks = Vec::new();
    let mut access_arms = Vec::new();
    let struct_check_calls = struct_checks.iter().map(|check| {
        quote! {
            {
                let mut valid = validator.__valid(#type_name, &mut errors);
                #check(self, &mut valid);
            }
        }
    });

    for field in &fields {
        let Some(field_ident) = field.ident.as_ref() else {
            continue;
        };
        let field_name = canonical(field_ident);
        let rules = parse_rules(&field.attrs)?;
        validate_field_targets(&rules, &field_names)?;
        collect_access_fields(&rules, &field_name, &mut access_fields);
        let target = quote! {
            #crate_path::FieldTarget::new(
                #type_name,
                #field_name,
                #field_name,
            )
        };
        reject_field_rules_inside_dive(&rules)?;
        let mut spec_index = 0;
        let field_checks = build_checks(
            rules,
            quote!(&self.#field_ident),
            target.clone(),
            target,
            &field.ty,
            false,
            &mut spec_index,
            &crate_path,
        )?;

        if !field_checks.preflight.is_empty() || !field_checks.execute.is_empty() {
            let preflight = field_checks.preflight;
            let execute = field_checks.execute;
            checks.push(quote! {
                {
                    #(#preflight)*
                    let mut skip_rest = false;
                    #(#execute)*
                }
            });
        }
    }

    for field in &fields {
        let Some(field_ident) = field.ident.as_ref() else {
            continue;
        };
        let field_name = canonical(field_ident);
        if access_fields.contains(&field_name) {
            access_arms.push(quote! {
                #field_name => Some(#crate_path::__private::FieldRef::new(#field_name, &self.#field_ident)),
            });
        }
    }

    Ok(quote! {
        impl #impl_generics #crate_path::Validate for #name #ty_generics #where_clause {
            fn validate(
                &self,
                validator: &#crate_path::Validator,
            ) -> std::result::Result<(), #crate_path::Error> {
                let context = #crate_path::__private::Context::new();
                self.__validate_with_context(validator, &context)
            }

            fn __validate_with_context(
                &self,
                validator: &#crate_path::Validator,
                context: &#crate_path::__private::Context,
            ) -> std::result::Result<(), #crate_path::Error> {
                let mut errors = Vec::new();

                #(#checks)*
                #(#struct_check_calls)*

                if errors.is_empty() {
                    Ok(())
                } else {
                    Err(#crate_path::Error::failed(errors))
                }
            }
        }

        impl #impl_generics #crate_path::__private::Access for #name #ty_generics #where_clause {
            fn field<'__validator>(
                &'__validator self,
                name: &'__validator str,
            ) -> Option<#crate_path::__private::FieldRef<'__validator>> {
                match name {
                    #(#access_arms)*
                    _ => None,
                }
            }
        }
    })
}

fn parse_struct_checks(attrs: &[syn::Attribute]) -> syn::Result<Vec<syn::Path>> {
    let mut checks = Vec::new();

    for attr in attrs {
        if !attr.path().is_ident("validate") {
            continue;
        }

        attr.parse_nested_meta(|meta| {
            if meta.path.is_ident("check") {
                let value = meta.value()?;
                let check: LitStr = value.parse()?;
                checks.push(check.parse()?);
                return Ok(());
            }

            Err(meta.error("unsupported struct-level validate attribute"))
        })?;
    }

    Ok(checks)
}

#[derive(Clone, Debug)]
enum RuleAttr {
    Rule {
        name: String,
        params: Vec<ParamAttr>,
    },
    Alias(String),
    FieldRule {
        name: String,
        params: Vec<ParamAttr>,
    },
    UniqueField {
        field: String,
        member: syn::Ident,
    },
    OmitEmpty,
    Nested,
    Dive(DiveAttr),
}

#[derive(Clone, Debug)]
enum ParamAttr {
    Positional(String),
    Named(String, String),
    List(String, Vec<String>),
}

#[derive(Clone, Debug)]
enum DiveAttr {
    Values(Vec<RuleAttr>),
    Map {
        keys: Vec<RuleAttr>,
        values: Vec<RuleAttr>,
    },
}

#[derive(Default)]
struct GeneratedChecks {
    preflight: Vec<proc_macro2::TokenStream>,
    execute: Vec<proc_macro2::TokenStream>,
}

fn exposes_value_access(rules: &[RuleAttr]) -> bool {
    let has_nested = rules.iter().any(|rule| matches!(rule, RuleAttr::Nested));

    rules.iter().any(|rule| match rule {
        RuleAttr::Rule { name, .. } => !(has_nested && name == "required"),
        RuleAttr::Alias(_) => true,
        RuleAttr::FieldRule { .. } => true,
        RuleAttr::UniqueField { .. } => false,
        RuleAttr::OmitEmpty => !has_nested,
        RuleAttr::Nested | RuleAttr::Dive(_) => false,
    })
}

fn collect_access_fields(rules: &[RuleAttr], current: &str, access_fields: &mut BTreeSet<String>) {
    if exposes_value_access(rules) {
        access_fields.insert(current.to_owned());
    }

    for rule in rules {
        if let RuleAttr::FieldRule { name, params } = rule {
            for target in field_targets(name, params) {
                access_fields.insert(target.to_owned());
            }
        }
    }
}

fn validate_field_targets(rules: &[RuleAttr], field_names: &BTreeSet<String>) -> syn::Result<()> {
    for rule in rules {
        if let RuleAttr::FieldRule { name, params } = rule {
            for target in field_targets(name, params) {
                if !field_names.contains(target) {
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

fn reject_field_rules_inside_dive(rules: &[RuleAttr]) -> syn::Result<()> {
    for rule in rules {
        match rule {
            RuleAttr::Dive(DiveAttr::Values(rules)) => reject_field_rules(rules)?,
            RuleAttr::Dive(DiveAttr::Map { keys, values }) => {
                reject_field_rules(keys)?;
                reject_field_rules(values)?;
            }
            RuleAttr::Rule { .. }
            | RuleAttr::Alias(_)
            | RuleAttr::FieldRule { .. }
            | RuleAttr::UniqueField { .. }
            | RuleAttr::OmitEmpty
            | RuleAttr::Nested => {}
        }
    }

    Ok(())
}

fn reject_field_rules(rules: &[RuleAttr]) -> syn::Result<()> {
    for rule in rules {
        match rule {
            RuleAttr::FieldRule { name, .. } => {
                return Err(syn::Error::new(
                    proc_macro2::Span::call_site(),
                    format!("validate rule '{name}' is not supported inside dive"),
                ));
            }
            RuleAttr::UniqueField { .. } => {
                return Err(syn::Error::new(
                    proc_macro2::Span::call_site(),
                    "unique=field is not supported inside dive",
                ));
            }
            RuleAttr::Dive(DiveAttr::Values(rules)) => reject_field_rules(rules)?,
            RuleAttr::Dive(DiveAttr::Map { keys, values }) => {
                reject_field_rules(keys)?;
                reject_field_rules(values)?;
            }
            RuleAttr::Rule { .. } | RuleAttr::Alias(_) | RuleAttr::OmitEmpty | RuleAttr::Nested => {
            }
        }
    }

    Ok(())
}

fn build_checks(
    rules: Vec<RuleAttr>,
    value: proc_macro2::TokenStream,
    target: proc_macro2::TokenStream,
    preflight_target: proc_macro2::TokenStream,
    value_type: &Type,
    type_only: bool,
    spec_index: &mut usize,
    crate_path: &proc_macro2::TokenStream,
) -> syn::Result<GeneratedChecks> {
    let has_nested = rules.iter().any(|rule| matches!(rule, RuleAttr::Nested));
    let is_option = is_option_type(value_type);
    let is_nested_option = has_nested && is_option;
    let mut checks = GeneratedChecks::default();

    for rule in rules {
        match rule {
            RuleAttr::Rule { name, params } => {
                if has_nested && name == "required" {
                    if is_option {
                        checks.execute.push(quote! {
                            if !skip_rest {
                                validator.__validate_required_option(
                                    &mut errors,
                                    #target,
                                    #value,
                                );
                            }
                        });
                    }
                    continue;
                }

                let inserts = build_params(&params);
                let spec = next_ident("spec", spec_index);
                let validate =
                    build_preflight(value_type, type_only, &preflight_target, &value, &spec);
                checks.preflight.push(quote! {
                    let #spec = {
                        let mut params = #crate_path::__private::RawParams::new();
                        #(#inserts)*
                        #crate_path::__private::Spec::with_params(#name, params)
                    };
                    #validate
                });
                checks.execute.push(quote! {{
                    if !skip_rest {
                        if validator.__validate_spec(
                            &mut errors,
                            #target,
                            #value,
                            #spec.clone(),
                            context,
                            self,
                        )? {
                            skip_rest = true;
                        }
                    }
                }});
            }
            RuleAttr::Alias(alias) => {
                let spec = next_ident("spec", spec_index);
                let validate =
                    build_preflight(value_type, type_only, &preflight_target, &value, &spec);
                checks.preflight.push(quote! {
                    let #spec = #crate_path::__private::Spec::with_params(
                        #alias,
                        #crate_path::__private::RawParams::new(),
                    );
                    #validate
                });
                checks.execute.push(quote! {{
                    if !skip_rest {
                        if validator.__validate_spec(
                            &mut errors,
                            #target,
                            #value,
                            #spec.clone(),
                            context,
                            self,
                        )? {
                            skip_rest = true;
                        }
                    }
                }});
            }
            RuleAttr::FieldRule { name, params } => {
                let inserts = build_params(&params);
                let spec = next_ident("spec", spec_index);
                let validate =
                    build_preflight(value_type, type_only, &preflight_target, &value, &spec);
                checks.preflight.push(quote! {
                    let #spec = {
                        let mut params = #crate_path::__private::RawParams::new();
                        #(#inserts)*
                        #crate_path::__private::Spec::with_params(#name, params)
                    };
                    #validate
                });
                checks.execute.push(quote! {{
                    if !skip_rest {
                        if validator.__validate_spec(
                            &mut errors,
                            #target,
                            #value,
                            #spec.clone(),
                            context,
                            self,
                        )? {
                            skip_rest = true;
                        }
                    }
                }});
            }
            RuleAttr::UniqueField { field, member } => {
                let Some(kind) = collection_kind(value_type, crate_path) else {
                    return Err(syn::Error::new(
                        member.span(),
                        "unique=field requires a Vec, array, or slice field",
                    ));
                };
                let spec = next_ident("spec", spec_index);
                let projection = next_ident("projection", spec_index);
                checks.preflight.push(quote! {
                    let #projection = #crate_path::__private::Projection::new(
                        &(#value)[..],
                        #field,
                        #kind,
                        |item| &item.#member,
                    );
                    let #spec = {
                        let mut params = #crate_path::__private::RawParams::new();
                        params.positional(#field);
                        #crate_path::__private::Spec::with_params("unique", params)
                    };
                    validator.__validate_item_params(
                        #preflight_target,
                        #spec.clone(),
                        context,
                        self,
                        &#projection,
                    )?;
                });
                checks.execute.push(quote! {{
                    if !skip_rest {
                        if validator.__validate_items(
                            &mut errors,
                            #target,
                            #spec.clone(),
                            context,
                            self,
                            &#projection,
                        )? {
                            skip_rest = true;
                        }
                    }
                }});
            }
            RuleAttr::OmitEmpty => {
                if is_nested_option {
                    checks.execute.push(quote! {
                        if !skip_rest && (#value).is_none() {
                            skip_rest = true;
                        }
                    });
                } else if !has_nested {
                    checks.execute.push(quote! {
                        if !skip_rest && validator.__skip_empty(#value) {
                            skip_rest = true;
                        }
                    });
                }
            }
            RuleAttr::Nested => {
                if is_option {
                    checks.execute.push(quote! {
                        if !skip_rest {
                            validator.__validate_nested_option(
                                &mut errors,
                                #target,
                                #value,
                                context,
                            )?;
                        }
                    });
                } else {
                    checks.execute.push(quote! {
                        if !skip_rest {
                            validator.__validate_nested(
                                &mut errors,
                                #target,
                                #value,
                                context,
                            )?;
                        }
                    });
                }
            }
            RuleAttr::Dive(dive) => match dive {
                DiveAttr::Values(rules) => {
                    let item_type = collection_item_type(value_type).ok_or_else(|| {
                        syn::Error::new_spanned(
                            value_type,
                            "dive requires a Vec, array, or slice field",
                        )
                    })?;
                    let element_checks = build_checks(
                        rules,
                        quote!(item),
                        quote!(item_target.clone()),
                        quote!((#preflight_target).index(0)),
                        item_type,
                        true,
                        spec_index,
                        crate_path,
                    )?;
                    let GeneratedChecks { preflight, execute } = element_checks;
                    checks.preflight.extend(preflight);

                    checks.execute.push(quote! {
                        if !skip_rest {
                            for (index, item) in (#value).iter().enumerate() {
                                let item_target = #target.index(index);
                                let mut skip_rest = false;
                                #(#execute)*
                            }
                        }
                    });
                }
                DiveAttr::Map { keys, values } => {
                    let key_type = map_key_type(value_type).ok_or_else(|| {
                        syn::Error::new_spanned(
                            value_type,
                            "map dive requires a HashMap or BTreeMap",
                        )
                    })?;
                    let value_type = map_value_type(value_type).expect("map key and value coexist");
                    let key_checks = build_checks(
                        keys,
                        quote!(key),
                        quote!(entry_target.clone()),
                        quote!((#preflight_target).clone()),
                        key_type,
                        true,
                        spec_index,
                        crate_path,
                    )?;
                    let value_checks = build_checks(
                        values,
                        quote!(value),
                        quote!(entry_target.clone()),
                        quote!((#preflight_target).clone()),
                        value_type,
                        true,
                        spec_index,
                        crate_path,
                    )?;
                    let GeneratedChecks {
                        preflight: key_preflight,
                        execute: key_execute,
                    } = key_checks;
                    let GeneratedChecks {
                        preflight: value_preflight,
                        execute: value_execute,
                    } = value_checks;
                    checks.preflight.extend(key_preflight);
                    checks.preflight.extend(value_preflight);

                    checks.execute.push(quote! {
                        if !skip_rest {
                            for (key, value) in (#value).iter() {
                                let entry_target = #target.key(key);
                                {
                                    let mut skip_rest = false;
                                    #(#key_execute)*
                                }
                                {
                                    let mut skip_rest = false;
                                    #(#value_execute)*
                                }
                            }
                        }
                    });
                }
            },
        }
    }

    Ok(checks)
}

fn build_preflight(
    value_type: &Type,
    type_only: bool,
    target: &proc_macro2::TokenStream,
    value: &proc_macro2::TokenStream,
    spec: &syn::Ident,
) -> proc_macro2::TokenStream {
    if type_only {
        quote! {
            validator.__validate_type_params::<#value_type, _>(
                #target,
                #spec.clone(),
                context,
                self,
            )?;
        }
    } else {
        quote! {
            validator.__validate_params(
                #target,
                #value,
                #spec.clone(),
                context,
                self,
            )?;
        }
    }
}

fn next_ident(prefix: &str, index: &mut usize) -> syn::Ident {
    let ident = format_ident!("__validator_{prefix}_{}", *index);
    *index += 1;
    ident
}

fn build_params(params: &[ParamAttr]) -> Vec<proc_macro2::TokenStream> {
    params
        .iter()
        .map(|param| match param {
            ParamAttr::Positional(value) => quote! {
                params.positional(#value);
            },
            ParamAttr::Named(name, value) => quote! {
                params.named(#name, #value);
            },
            ParamAttr::List(name, values) => quote! {
                params.named_list(#name, vec![#(#values.to_owned()),*]);
            },
        })
        .collect()
}

fn parse_rules(attrs: &[syn::Attribute]) -> syn::Result<Vec<RuleAttr>> {
    let mut rules = Vec::new();

    for attr in attrs {
        if !attr.path().is_ident("validate") {
            continue;
        }

        attr.parse_nested_meta(|meta| parse_rule_meta(meta, &mut rules))?;
    }

    Ok(rules)
}

fn parse_rule_meta(meta: ParseNestedMeta<'_>, rules: &mut Vec<RuleAttr>) -> syn::Result<()> {
    if meta.path.is_ident("dive") {
        rules.push(RuleAttr::Dive(parse_dive_rules(meta)?));
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
                params: parse_conditional_pairs(meta, name)?,
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
                    parse_field_list(meta, name)?,
                )],
            });
            return Ok(());
        }
    }

    if meta.path.is_ident("unique") && meta.input.peek(Token![=]) {
        let value = meta.value()?;
        let field: LitStr = value.parse()?;
        if field.value().contains('.') {
            return Err(syn::Error::new_spanned(
                field,
                "unique=field only supports a direct element field",
            ));
        }
        let member = member(&field)?;
        rules.push(RuleAttr::UniqueField {
            field: canonical(&member),
            member,
        });
        return Ok(());
    }
    if meta.path.is_ident("unique") && meta.input.peek(syn::token::Paren) {
        return Err(meta.error("unique field syntax is `unique = \"field\"`"));
    }

    let Some(name) = meta.path.get_ident().map(canonical) else {
        return Err(meta.error("validate rule must be a single identifier"));
    };
    rules.push(rule(name, parse_custom_params(meta)?));
    Ok(())
}

fn parse_custom_params(meta: ParseNestedMeta<'_>) -> syn::Result<Vec<ParamAttr>> {
    if meta.input.peek(Token![=]) {
        return Ok(vec![ParamAttr::Positional(parse_param_value(
            meta.value()?,
        )?)]);
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
                parse_param_value(&content)?,
            ));
        } else {
            params.push(ParamAttr::Positional(parse_param_value(&content)?));
        }

        if content.peek(Token![,]) {
            content.parse::<Token![,]>()?;
        } else if !content.is_empty() {
            return Err(content.error("expected ',' between validate parameters"));
        }
    }

    Ok(params)
}

fn parse_dive_rules(meta: ParseNestedMeta<'_>) -> syn::Result<DiveAttr> {
    let mut rules = Vec::new();
    let mut keys = None;
    let mut values = None;

    meta.parse_nested_meta(|nested| {
        if nested.path.is_ident("keys") {
            if keys.is_some() {
                return Err(nested.error("duplicate keys(...) in dive"));
            }
            keys = Some(parse_dive_section(nested)?);
            return Ok(());
        }

        if nested.path.is_ident("values") {
            if values.is_some() {
                return Err(nested.error("duplicate values(...) in dive"));
            }
            values = Some(parse_dive_section(nested)?);
            return Ok(());
        }

        parse_rule_meta(nested, &mut rules)
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

fn parse_dive_section(meta: ParseNestedMeta<'_>) -> syn::Result<Vec<RuleAttr>> {
    let mut rules = Vec::new();
    meta.parse_nested_meta(|nested| parse_rule_meta(nested, &mut rules))?;

    if rules.is_empty() {
        return Err(meta.error("map dive section requires at least one rule"));
    }

    Ok(rules)
}

fn parse_conditional_pairs(meta: ParseNestedMeta<'_>, rule: &str) -> syn::Result<Vec<ParamAttr>> {
    let mut params = Vec::new();
    let mut seen = BTreeSet::new();

    meta.parse_nested_meta(|nested| {
        let Some(field) = nested.path.get_ident().map(canonical) else {
            return Err(nested.error("conditional rule field must be a field identifier"));
        };

        if !seen.insert(field.clone()) {
            return Err(nested.error(format!("duplicate field '{field}' in {rule}")));
        }

        params.push(ParamAttr::Named(field, parse_param_value(nested.value()?)?));
        Ok(())
    })?;

    if params.is_empty() {
        return Err(meta.error(format!("{rule} requires at least one field condition")));
    }

    Ok(params)
}

fn parse_field_list(meta: ParseNestedMeta<'_>, rule: &str) -> syn::Result<Vec<String>> {
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

fn is_option_type(ty: &Type) -> bool {
    let Type::Path(path) = ty else {
        return false;
    };

    path.path.segments.last().is_some_and(|segment| {
        segment.ident == "Option" && matches!(segment.arguments, PathArguments::AngleBracketed(_))
    })
}

fn collection_item_type(ty: &Type) -> Option<&Type> {
    match ty {
        Type::Path(path) => path.path.segments.last().and_then(|segment| {
            if segment.ident != "Vec" {
                return None;
            }

            let PathArguments::AngleBracketed(generic_args) = &segment.arguments else {
                return None;
            };

            generic_args
                .args
                .iter()
                .find_map(|generic_param| match generic_param {
                    GenericArgument::Type(ty) => Some(ty),
                    _ => None,
                })
        }),
        Type::Array(TypeArray { elem, .. }) => Some(elem.as_ref()),
        Type::Reference(TypeReference { elem, .. }) => match elem.as_ref() {
            Type::Slice(slice) => Some(slice.elem.as_ref()),
            _ => None,
        },
        _ => None,
    }
}

fn map_key_type(ty: &Type) -> Option<&Type> {
    map_type(ty, 0)
}

fn map_value_type(ty: &Type) -> Option<&Type> {
    map_type(ty, 1)
}

fn map_type(ty: &Type, index: usize) -> Option<&Type> {
    let Type::Path(path) = ty else {
        return None;
    };

    path.path.segments.last().and_then(|segment| {
        if segment.ident != "HashMap" && segment.ident != "BTreeMap" {
            return None;
        }

        let PathArguments::AngleBracketed(generic_args) = &segment.arguments else {
            return None;
        };

        generic_args
            .args
            .iter()
            .filter_map(|generic_param| match generic_param {
                GenericArgument::Type(ty) => Some(ty),
                _ => None,
            })
            .nth(index)
    })
}

fn collection_kind(
    ty: &Type,
    crate_path: &proc_macro2::TokenStream,
) -> Option<proc_macro2::TokenStream> {
    match ty {
        Type::Path(path)
            if path
                .path
                .segments
                .last()
                .is_some_and(|segment| segment.ident == "Vec") =>
        {
            Some(quote!(#crate_path::Kind::Vec))
        }
        Type::Array(_) => Some(quote!(#crate_path::Kind::Array)),
        Type::Reference(TypeReference { elem, .. }) if matches!(elem.as_ref(), Type::Slice(_)) => {
            Some(quote!(#crate_path::Kind::Slice))
        }
        _ => None,
    }
}

fn validator_crate_path() -> syn::Result<proc_macro2::TokenStream> {
    match crate_name("validator") {
        Ok(FoundCrate::Itself) => Ok(quote!(::validator)),
        Ok(FoundCrate::Name(name)) => {
            let name = syn::Ident::new(&name.replace('-', "_"), proc_macro2::Span::call_site());
            Ok(quote!(::#name))
        }
        Err(error) => Err(syn::Error::new(
            proc_macro2::Span::call_site(),
            format!("validator crate could not be resolved: {error}"),
        )),
    }
}

fn canonical(ident: &syn::Ident) -> String {
    ident.unraw().to_string()
}

fn member(field: &LitStr) -> syn::Result<syn::Ident> {
    let name = field.value();
    syn::parse_str::<syn::Ident>(&name)
        .or_else(|_| syn::parse_str::<syn::Ident>(&format!("r#{name}")))
        .map_err(|_| syn::Error::new_spanned(field, "unique field must be a Rust field identifier"))
}

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

fn rule(name: impl Into<String>, params: Vec<ParamAttr>) -> RuleAttr {
    RuleAttr::Rule {
        name: name.into(),
        params,
    }
}

fn parse_param_value(input: syn::parse::ParseStream<'_>) -> syn::Result<String> {
    let expr: Expr = input.parse()?;
    match expr {
        Expr::Lit(ExprLit { lit, .. }) => parse_lit(lit),
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

fn parse_lit(lit: Lit) -> syn::Result<String> {
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
