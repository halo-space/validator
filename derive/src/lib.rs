mod model;
mod parse;
mod types;

use proc_macro::TokenStream;
use proc_macro_crate::{FoundCrate, crate_name};
use quote::{format_ident, quote};
use std::collections::{BTreeMap, BTreeSet};
use syn::ext::IdentExt;
use syn::{Data, DeriveInput, Fields, LitStr, Type, parse_macro_input};

use self::parse::rules as parse_rules;
use self::model::{DiveAttr, GeneratedChecks, ParamAttr, RuleAttr};
use self::types::{collection_item, collection_kind, field_kind, is_option, map_key, map_value};

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
    let field_members = fields
        .iter()
        .filter_map(|field| field.ident.as_ref())
        .map(|ident| (canonical(ident), ident.clone()))
        .collect::<BTreeMap<_, _>>();
    let mut access_fields = BTreeSet::new();
    let mut access_paths = BTreeMap::new();

    let mut checks = Vec::new();
    let mut access_arms = Vec::new();
    let kind_arms = fields
        .iter()
        .filter_map(|field| {
            let ident = field.ident.as_ref()?;
            let name = canonical(ident);
            let kind = field_kind(&field.ty, quote!(&self.#ident), &crate_path);
            Some(quote!(#name => #kind,))
        })
        .collect::<Vec<_>>();
    let kind_declaration = (!struct_checks.is_empty()).then(|| {
        quote! {
            let field_kind = |field: &str| -> #crate_path::Kind { match field {
                #(#kind_arms)*
                _ => #crate_path::Kind::Other,
            }};
        }
    });
    let struct_check_calls = struct_checks.iter().map(|check| {
        quote! {
            if context.active() {
                let error_start = errors.len();
                let mut valid = validator.__valid(#type_name, &mut errors, &field_kind);
                #check(self, &mut valid);
                validator.__retain_selected_struct_errors(
                    &mut errors,
                    error_start,
                    context,
                );
            }
        }
    });

    for field in &fields {
        let Some(field_ident) = field.ident.as_ref() else {
            continue;
        };
        let field_name = canonical(field_ident);
        let rules = parse_rules(&field.attrs)?;
        collect_access(
            &rules,
            field_ident,
            &field_members,
            &mut access_fields,
            &mut access_paths,
        )?;
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

        let preflight = field_checks.preflight;
        let execute = field_checks.execute;
        checks.push(quote! {
            if context.includes(#field_name) {
                #(#preflight)*
                let mut skip_rest = false;
                #(#execute)*
            }
        });
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

    for (path, segments) in access_paths {
        let first = segments
            .first()
            .expect("validated field path must contain a segment");
        let mut resolve = quote! {
            #crate_path::__private::Segment::new(&self.#first).resolve()
        };
        for segment in &segments[1..] {
            resolve = quote! {
                #resolve.and_then(|value| {
                    #crate_path::__private::Segment::new(&value.#segment).resolve()
                })
            };
        }

        access_arms.push(quote! {
            #path => {
                use #crate_path::__private::Resolve as _;
                #resolve.map(|value| #crate_path::__private::FieldRef::new(#path, value))
            },
        });
    }

    Ok(quote! {
        impl #impl_generics #crate_path::Validate for #name #ty_generics #where_clause {
            fn validate(
                &self,
                validator: &#crate_path::Validator,
            ) -> std::result::Result<(), #crate_path::Error> {
                let context = #crate_path::__private::Context::new();
                <Self as #crate_path::__private::Selective>::__validate_with_context(
                    self,
                    validator,
                    &context,
                )
            }
        }

        impl #impl_generics #crate_path::__private::Selective for #name #ty_generics #where_clause {
            fn __validate_with_context(
                &self,
                validator: &#crate_path::Validator,
                context: &#crate_path::__private::Context<'_>,
            ) -> std::result::Result<(), #crate_path::Error> {
                let mut errors = Vec::new();

                #(#checks)*
                #kind_declaration
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

fn collect_access(
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

fn parse_field_path(rule: &str, path: &str) -> syn::Result<Vec<syn::Ident>> {
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
            | RuleAttr::UniqueFields { .. }
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
            RuleAttr::UniqueFields { .. } => {
                return Err(syn::Error::new(
                    proc_macro2::Span::call_site(),
                    "parameterized unique is not supported inside dive",
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
    let is_option = is_option(value_type);
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
            RuleAttr::UniqueFields { paths } => {
                let Some(kind) = collection_kind(value_type, crate_path) else {
                    return Err(syn::Error::new(
                        proc_macro2::Span::call_site(),
                        "parameterized unique requires a Vec, array, or slice field",
                    ));
                };
                let spec = next_ident("spec", spec_index);
                let projection = next_ident("projection", spec_index);
                let names = paths
                    .iter()
                    .map(|path| path.name.as_str())
                    .collect::<Vec<_>>();
                let arms = paths.iter().map(|path| {
                    let name = &path.name;
                    let segments = &path.segments;
                    if segments.len() == 1 {
                        let member = &segments[0];
                        return quote! {
                            #name => Some(&item.#member as &dyn #crate_path::Value),
                        };
                    }

                    let first = &segments[0];
                    let terminal = segments
                        .last()
                        .expect("item path must contain a terminal segment");
                    let mut resolve = quote! {
                        #crate_path::__private::Segment::new(&item.#first).resolve()
                    };
                    for segment in &segments[1..segments.len() - 1] {
                        resolve = quote! {
                            #resolve.and_then(|value| {
                                #crate_path::__private::Segment::new(&value.#segment).resolve()
                            })
                        };
                    }

                    quote! {
                        #name => {
                            use #crate_path::__private::Resolve as _;
                            #resolve.map(|value| &value.#terminal as &dyn #crate_path::Value)
                        },
                    }
                });
                checks.preflight.push(quote! {
                    let #projection = #crate_path::__private::Projection::new(
                        &(#value)[..],
                        &[#(#names),*],
                        #kind,
                        |item, field| match field {
                            #(#arms)*
                            _ => None,
                        },
                    );
                    let #spec = {
                        let mut params = #crate_path::__private::RawParams::new();
                        #(params.positional(#names);)*
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
                    let item_type = collection_item(value_type).ok_or_else(|| {
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
                                if context.includes(item_target.struct_field_name.as_ref()) {
                                    let mut skip_rest = false;
                                    #(#execute)*
                                }
                            }
                        }
                    });
                }
                DiveAttr::Map { keys, values } => {
                    let key_type = map_key(value_type).ok_or_else(|| {
                        syn::Error::new_spanned(
                            value_type,
                            "map dive requires a HashMap or BTreeMap",
                        )
                    })?;
                    let value_type = map_value(value_type).expect("map key and value coexist");
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
                                if context.includes(entry_target.struct_field_name.as_ref()) {
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

fn member_name(name: &str) -> Option<syn::Ident> {
    syn::parse_str::<syn::Ident>(name)
        .or_else(|_| syn::parse_str::<syn::Ident>(&format!("r#{name}")))
        .ok()
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
