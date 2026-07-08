use proc_macro::TokenStream;
use quote::quote;
use std::collections::BTreeSet;
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
    let DeriveInput {
        attrs,
        data,
        generics,
        ident: name,
        ..
    } = input;
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
        .map(ToString::to_string)
        .collect::<BTreeSet<_>>();
    let mut access_fields = BTreeSet::new();

    let mut checks = Vec::new();
    let mut access_arms = Vec::new();
    let struct_check_calls = struct_checks.iter().map(|check| {
        quote! {
            {
                let mut valid = validator.__valid(stringify!(#name), &mut errors);
                #check(self, &mut valid);
            }
        }
    });

    for field in &fields {
        let is_option = is_option_type(&field.ty);
        let item_is_option = collection_item_type(&field.ty).is_some_and(is_option_type);
        let map_value_is_option = map_value_type(&field.ty).is_some_and(is_option_type);
        let collection_kind = collection_kind(&field.ty);
        let Some(field_ident) = field.ident.as_ref() else {
            continue;
        };
        let field_name = field_ident.to_string();
        let rules = parse_rules(&field.attrs)?;
        validate_compare_targets(&rules, &field_names)?;
        collect_access_fields(&rules, &field_name, &mut access_fields);
        let target = quote! {
            ::validator::FieldTarget::new(
                stringify!(#name),
                #field_name,
                #field_name,
            )
        };
        reject_compare_fields_inside_dive(&rules)?;
        let field_checks = build_checks(
            rules,
            quote!(&self.#field_ident),
            target,
            is_option,
            Some(item_is_option),
            Some(map_value_is_option),
            collection_kind,
        )?;

        if !field_checks.is_empty() {
            checks.push(quote! {
                {
                    let mut skip_rest = false;
                    #(#field_checks)*
                }
            });
        }
    }

    for field in &fields {
        let Some(field_ident) = field.ident.as_ref() else {
            continue;
        };
        let field_name = field_ident.to_string();
        if access_fields.contains(&field_name) {
            access_arms.push(quote! {
                #field_name => Some(::validator::__private::FieldRef::new(#field_name, &self.#field_ident)),
            });
        }
    }

    Ok(quote! {
        impl #impl_generics ::validator::Validate for #name #ty_generics #where_clause {
            fn validate(
                &self,
                validator: &::validator::Validator,
            ) -> std::result::Result<(), ::validator::Error> {
                let context = ::validator::__private::Context::new();
                self.__validate_with_context(validator, &context)
            }

            fn __validate_with_context(
                &self,
                validator: &::validator::Validator,
                context: &::validator::__private::Context,
            ) -> std::result::Result<(), ::validator::Error> {
                let mut errors = Vec::new();

                #(#checks)*
                #(#struct_check_calls)*

                if errors.is_empty() {
                    Ok(())
                } else {
                    Err(::validator::Error::failed(errors))
                }
            }
        }

        impl #impl_generics ::validator::__private::Access for #name #ty_generics #where_clause {
            fn field(&self, name: &str) -> Option<::validator::__private::FieldRef<'_>> {
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
        params: Vec<(String, String)>,
    },
    Alias(String),
    CompareField { name: String, target: String },
    OmitEmpty,
    Nested,
    Dive(DiveAttr),
}

#[derive(Clone, Debug)]
enum DiveAttr {
    Values(Vec<RuleAttr>),
    Map {
        keys: Vec<RuleAttr>,
        values: Vec<RuleAttr>,
    },
}

#[derive(Clone, Debug)]
enum UniqueCollection {
    Items(proc_macro2::TokenStream),
    MapValues,
}

impl UniqueCollection {
    fn kind(&self) -> proc_macro2::TokenStream {
        match self {
            Self::Items(kind) => kind.clone(),
            Self::MapValues => quote!(::validator::Kind::Map),
        }
    }

    fn values(&self, value: &proc_macro2::TokenStream) -> proc_macro2::TokenStream {
        match self {
            Self::Items(_) => quote!((#value).iter()),
            Self::MapValues => quote!((#value).values()),
        }
    }
}

fn exposes_value_access(rules: &[RuleAttr]) -> bool {
    let has_nested = rules.iter().any(|rule| matches!(rule, RuleAttr::Nested));

    rules.iter().any(|rule| match rule {
        RuleAttr::Rule { name, .. } => !(has_nested && name == "required") && name != "unique",
        RuleAttr::Alias(_) => true,
        RuleAttr::CompareField { .. } => true,
        RuleAttr::OmitEmpty => !has_nested,
        RuleAttr::Nested | RuleAttr::Dive(_) => false,
    })
}

fn collect_access_fields(rules: &[RuleAttr], current: &str, access_fields: &mut BTreeSet<String>) {
    if exposes_value_access(rules) {
        access_fields.insert(current.to_owned());
    }

    for rule in rules {
        if let RuleAttr::CompareField { target, .. } = rule {
            access_fields.insert(target.clone());
        }
    }
}

fn validate_compare_targets(rules: &[RuleAttr], field_names: &BTreeSet<String>) -> syn::Result<()> {
    for rule in rules {
        if let RuleAttr::CompareField { name, target } = rule
            && !field_names.contains(target)
        {
            return Err(syn::Error::new(
                proc_macro2::Span::call_site(),
                format!("validate rule '{name}' references unknown field '{target}'"),
            ));
        }
    }

    Ok(())
}

fn reject_compare_fields_inside_dive(rules: &[RuleAttr]) -> syn::Result<()> {
    for rule in rules {
        match rule {
            RuleAttr::Dive(DiveAttr::Values(rules)) => reject_compare_fields(rules)?,
            RuleAttr::Dive(DiveAttr::Map { keys, values }) => {
                reject_compare_fields(keys)?;
                reject_compare_fields(values)?;
            }
            RuleAttr::Rule { .. }
            | RuleAttr::Alias(_)
            | RuleAttr::CompareField { .. }
            | RuleAttr::OmitEmpty
            | RuleAttr::Nested => {}
        }
    }

    Ok(())
}

fn reject_compare_fields(rules: &[RuleAttr]) -> syn::Result<()> {
    for rule in rules {
        match rule {
            RuleAttr::CompareField { name, .. } => {
                return Err(syn::Error::new(
                    proc_macro2::Span::call_site(),
                    format!("validate rule '{name}' is not supported inside dive"),
                ));
            }
            RuleAttr::Dive(DiveAttr::Values(rules)) => reject_compare_fields(rules)?,
            RuleAttr::Dive(DiveAttr::Map { keys, values }) => {
                reject_compare_fields(keys)?;
                reject_compare_fields(values)?;
            }
            RuleAttr::Rule { .. } | RuleAttr::Alias(_) | RuleAttr::OmitEmpty | RuleAttr::Nested => {}
        }
    }

    Ok(())
}

fn build_checks(
    rules: Vec<RuleAttr>,
    value: proc_macro2::TokenStream,
    target: proc_macro2::TokenStream,
    is_option: bool,
    dive_item_is_option: Option<bool>,
    map_value_is_option: Option<bool>,
    collection_kind: Option<UniqueCollection>,
) -> syn::Result<Vec<proc_macro2::TokenStream>> {
    let has_nested = rules.iter().any(|rule| matches!(rule, RuleAttr::Nested));
    let is_nested_option = has_nested && is_option;
    let mut checks = Vec::new();

    for rule in rules {
        match rule {
            RuleAttr::Rule { name, params } => {
                if name == "unique" && let Some(collection) = collection_kind.clone() {
                    let kind = collection.kind();
                    let values = collection.values(&value);
                    checks.push(quote! {
                        if !skip_rest {
                            validator.__validate_unique_items(
                                &mut errors,
                                #target,
                                #kind,
                                #values,
                            );
                        }
                    });
                    continue;
                }

                if has_nested && name == "required" {
                    if is_option {
                        checks.push(quote! {
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

                let inserts = params.iter().map(|(key, value)| {
                    quote! {
                        params.insert(#key, #value);
                    }
                });
                checks.push(quote! {
                    if !skip_rest {
                        let mut params = ::validator::Params::new();
                        #(#inserts)*
                        validator.__validate_rule(
                            &mut errors,
                            #target,
                            #value,
                            #name,
                            params,
                            context,
                        )?;
                    }
                });
            }
            RuleAttr::Alias(alias) => {
                checks.push(quote! {
                    if !skip_rest {
                        validator.__validate_alias(
                            &mut errors,
                            #target,
                            #value,
                            #alias,
                            context,
                        )?;
                    }
                });
            }
            RuleAttr::CompareField {
                name,
                target: compare_name,
            } => {
                checks.push(quote! {
                    if !skip_rest {
                        let compare_field = ::validator::__private::Access::field(self, #compare_name);
                        validator.__validate_compare_field(
                            &mut errors,
                            #target,
                            #value,
                            compare_field.as_ref().map(|field| field.value()),
                            #name,
                            #compare_name,
                        );
                    }
                });
            }
            RuleAttr::OmitEmpty => {
                if is_nested_option {
                    checks.push(quote! {
                        if !skip_rest && (#value).is_none() {
                            skip_rest = true;
                        }
                    });
                } else if !has_nested {
                    checks.push(quote! {
                        if !skip_rest && validator.__skip_empty(#value) {
                            skip_rest = true;
                        }
                    });
                }
            }
            RuleAttr::Nested => {
                if is_option {
                    checks.push(quote! {
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
                    checks.push(quote! {
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
                    let item_is_option = dive_item_is_option.unwrap_or(false);
                    let element_checks = build_checks(
                        rules,
                        quote!(item),
                        quote!(item_target.clone()),
                        item_is_option,
                        None,
                        None,
                        None,
                    )?;

                    checks.push(quote! {
                        if !skip_rest {
                            for (index, item) in (#value).iter().enumerate() {
                                let item_target = #target.index(index);
                                let mut skip_rest = false;
                                #(#element_checks)*
                            }
                        }
                    });
                }
                DiveAttr::Map { keys, values } => {
                    let value_is_option = map_value_is_option.unwrap_or(false);
                    let key_checks = build_checks(
                        keys,
                        quote!(key),
                        quote!(entry_target.clone()),
                        false,
                        None,
                        None,
                        None,
                    )?;
                    let value_checks = build_checks(
                        values,
                        quote!(value),
                        quote!(entry_target.clone()),
                        value_is_option,
                        None,
                        None,
                        None,
                    )?;

                    checks.push(quote! {
                        if !skip_rest {
                            for (key, value) in (#value).iter() {
                                let entry_target = #target.key(key);
                                {
                                    let mut skip_rest = false;
                                    #(#key_checks)*
                                }
                                {
                                    let mut skip_rest = false;
                                    #(#value_checks)*
                                }
                            }
                        }
                    });
                }
            }
        }
    }

    Ok(checks)
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

    for name in COMPARE_FIELD_RULES {
        if meta.path.is_ident(*name) {
            let value = meta.value()?;
            let target: LitStr = value.parse()?;
            rules.push(RuleAttr::CompareField {
                name: (*name).to_owned(),
                target: target.value(),
            });
            return Ok(());
        }
    }

    if meta.path.is_ident("min") {
        let value = meta.value()?;
        rules.push(rule("min", vec![("min".to_owned(), parse_param_value(value)?)]));
        return Ok(());
    }

    if meta.path.is_ident("max") {
        let value = meta.value()?;
        rules.push(rule("max", vec![("max".to_owned(), parse_param_value(value)?)]));
        return Ok(());
    }

    if meta.path.is_ident("eq") {
        rules.push(rule("eq", parse_optional_value_param(meta)?));
        return Ok(());
    }

    if meta.path.is_ident("ne") {
        rules.push(rule("ne", parse_optional_value_param(meta)?));
        return Ok(());
    }

    if meta.path.is_ident("gt") {
        rules.push(rule("gt", parse_optional_value_param(meta)?));
        return Ok(());
    }

    if meta.path.is_ident("gte") {
        rules.push(rule("gte", parse_optional_value_param(meta)?));
        return Ok(());
    }

    if meta.path.is_ident("lt") {
        rules.push(rule("lt", parse_optional_value_param(meta)?));
        return Ok(());
    }

    if meta.path.is_ident("lte") {
        rules.push(rule("lte", parse_optional_value_param(meta)?));
        return Ok(());
    }

    if meta.path.is_ident("length") {
        rules.push(rule(
            "length",
            parse_keyed_params(meta, &["min", "max", "exact"])?,
        ));
        return Ok(());
    }

    if meta.path.is_ident("range") {
        rules.push(rule("range", parse_keyed_params(meta, &["min", "max"])?));
        return Ok(());
    }

    if meta.path.is_ident("regex") {
        rules.push(rule("regex", parse_keyed_params(meta, &["pattern"])?));
        return Ok(());
    }

    if meta.path.is_ident("oneof") {
        rules.push(rule(
            "oneof",
            vec![("values".to_owned(), parse_choice_values(meta, "oneof")?)],
        ));
        return Ok(());
    }

    if meta.path.is_ident("noneof") {
        rules.push(rule(
            "noneof",
            vec![("values".to_owned(), parse_choice_values(meta, "noneof")?)],
        ));
        return Ok(());
    }

    for name in MARKER_RULES {
        if meta.path.is_ident(*name) {
            rules.push(rule(*name, Vec::new()));
            return Ok(());
        }
    }

    for name in STRING_PARAM_RULES {
        if meta.path.is_ident(*name) {
            rules.push(rule(*name, parse_keyed_params(meta, &["value"])?));
            return Ok(());
        }
    }

    Err(meta.error("unsupported validate rule"))
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

fn parse_optional_value_param(meta: ParseNestedMeta<'_>) -> syn::Result<Vec<(String, String)>> {
    if meta.input.is_empty() {
        return Ok(Vec::new());
    }

    let value = meta.value()?;
    Ok(vec![("value".to_owned(), parse_param_value(value)?)])
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

            generic_args.args.iter().find_map(|generic_param| match generic_param {
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

fn collection_kind(ty: &Type) -> Option<UniqueCollection> {
    match ty {
        Type::Path(path) => path.path.segments.last().and_then(|segment| {
            if segment.ident == "Vec" {
                return Some(UniqueCollection::Items(quote!(::validator::Kind::Vec)));
            }

            if segment.ident == "HashMap" || segment.ident == "BTreeMap" {
                return Some(UniqueCollection::MapValues);
            }

            None
        }),
        Type::Array(_) => Some(UniqueCollection::Items(quote!(::validator::Kind::Array))),
        Type::Reference(TypeReference { elem, .. }) => match elem.as_ref() {
            Type::Slice(_) => Some(UniqueCollection::Items(quote!(::validator::Kind::Slice))),
            _ => None,
        },
        _ => None,
    }
}

fn map_value_type(ty: &Type) -> Option<&Type> {
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
            .nth(1)
    })
}

const MARKER_RULES: &[&str] = &[
    "email",
    "url",
    "uri",
    "http_url",
    "https_url",
    "ip",
    "ipv4",
    "ipv6",
    "cidr",
    "cidrv4",
    "cidrv6",
    "hostname",
    "hostname_rfc1123",
    "fqdn",
    "port",
    "uuid",
    "uuid3",
    "uuid4",
    "uuid5",
    "ulid",
    "json",
    "datetime",
    "ascii",
    "alpha",
    "alphanum",
    "numeric",
    "number",
    "lowercase",
    "uppercase",
    "boolean",
    "unique",
    "hexcolor",
    "rgb",
    "rgba",
    "hsl",
    "hsla",
    "cmyk",
];

const STRING_PARAM_RULES: &[&str] = &["contains", "containsany", "startswith", "endswith"];

const COMPARE_FIELD_RULES: &[&str] = &[
    "eq_field",
    "ne_field",
    "gt_field",
    "gte_field",
    "lt_field",
    "lte_field",
];

fn rule(name: impl Into<String>, params: Vec<(String, String)>) -> RuleAttr {
    RuleAttr::Rule {
        name: name.into(),
        params,
    }
}

fn parse_keyed_params(
    meta: ParseNestedMeta<'_>,
    allowed: &[&str],
) -> syn::Result<Vec<(String, String)>> {
    let mut params = Vec::new();

    meta.parse_nested_meta(|nested| {
        for name in allowed {
            if nested.path.is_ident(name) {
                params.push(((*name).to_owned(), parse_param_value(nested.value()?)?));
                return Ok(());
            }
        }

        Err(nested.error("unsupported validate parameter"))
    })?;

    Ok(params)
}

fn parse_choice_values(meta: ParseNestedMeta<'_>, rule: &str) -> syn::Result<String> {
    let content;
    parenthesized!(content in meta.input);

    let mut values = Vec::new();
    while !content.is_empty() {
        values.push(parse_choice_value(&content)?);

        if content.peek(Token![,]) {
            content.parse::<Token![,]>()?;
        }
    }

    if values.is_empty() {
        return Err(meta.error(format!("{rule} requires at least one value")));
    }

    Ok(values.join(","))
}

fn parse_choice_value(input: syn::parse::ParseStream<'_>) -> syn::Result<String> {
    let expr: Expr = input.parse()?;

    match expr {
        Expr::Lit(ExprLit {
            lit: Lit::Str(value),
            ..
        }) => Ok(value.value()),
        Expr::Lit(ExprLit {
            lit: Lit::Int(value),
            ..
        }) => Ok(value.base10_digits().to_owned()),
        Expr::Unary(ExprUnary {
            op: UnOp::Neg(_),
            expr,
            ..
        }) => match *expr {
            Expr::Lit(ExprLit {
                lit: Lit::Int(value),
                ..
            }) => Ok(format!("-{}", value.base10_digits())),
            expr => Err(syn::Error::new_spanned(
                expr,
                "choice validate value must be a string or integer literal",
            )),
        },
        expr => Err(syn::Error::new_spanned(
            expr,
            "choice validate value must be a string or integer literal",
        )),
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
