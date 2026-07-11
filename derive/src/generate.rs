use quote::{format_ident, quote};
use syn::Type;

use super::model::{DiveAttr, GeneratedChecks, ParamAttr, RuleAttr};
use super::types::{collection_item, collection_kind, is_option, map_key, map_value};

pub(super) fn build_checks(
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
                                if context.includes(item_target.struct_field_name()) {
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
                                if context.includes(entry_target.struct_field_name()) {
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
