mod access;
mod crate_path;
mod generate;
mod ident;
mod model;
mod parse;
mod types;
mod validate;

use proc_macro::TokenStream;
use quote::quote;
use std::collections::{BTreeMap, BTreeSet};
use syn::{Data, DeriveInput, Fields, LitStr, parse_macro_input};

use self::access::collect_access;
use self::crate_path::validator_crate_path;
use self::generate::build_checks;
use self::ident::canonical;
use self::parse::rules as parse_rules;
use self::types::field_kind;
use self::validate::reject_field_rules_inside_dive;

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
            #crate_path::__private::FieldTarget::new(
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
