use proc_macro::TokenStream;
use quote::quote;
use syn::meta::ParseNestedMeta;
use syn::{
    Data, DeriveInput, Expr, ExprLit, ExprUnary, Fields, Lit, LitStr, Token, UnOp, parenthesized,
    parse_macro_input,
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
    let name = input.ident;
    let fields = match input.data {
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

    let mut checks = Vec::new();

    for field in fields {
        let Some(field_ident) = field.ident else {
            continue;
        };
        let field_name = field_ident.to_string();
        let rules = parse_rules(&field.attrs)?;
        let target = quote! {
            ::validator::FieldTarget::new(
                stringify!(#name),
                #field_name,
                #field_name,
            )
        };
        let mut field_checks = Vec::new();

        for rule in rules {
            match rule {
                RuleAttr::Rule { name, args } => {
                    let inserts = args.iter().map(|(key, value)| {
                        quote! {
                            args.insert(#key, #value);
                        }
                    });
                    field_checks.push(quote! {
                        if !skip_rest {
                            let mut args = ::validator::Args::new();
                            #(#inserts)*
                            validator.__validate_rule(
                                &mut errors,
                                #target,
                                &self.#field_ident,
                                #name,
                                args,
                            );
                        }
                    });
                }
                RuleAttr::Alias(alias) => {
                    field_checks.push(quote! {
                        if !skip_rest {
                            validator.__validate_alias(
                                &mut errors,
                                #target,
                                &self.#field_ident,
                                #alias,
                            );
                        }
                    });
                }
                RuleAttr::OmitEmpty => {
                    field_checks.push(quote! {
                        if !skip_rest && validator.__skip_empty(&self.#field_ident) {
                            skip_rest = true;
                        }
                    });
                }
            }
        }

        if !field_checks.is_empty() {
            checks.push(quote! {
                {
                    let mut skip_rest = false;
                    #(#field_checks)*
                }
            });
        }
    }

    Ok(quote! {
        impl ::validator::Validate for #name {
            fn validate(
                &self,
                validator: &::validator::Validator,
            ) -> std::result::Result<(), ::validator::Errors> {
                let mut errors = ::validator::Errors::new();

                #(#checks)*

                if errors.is_empty() {
                    Ok(())
                } else {
                    Err(errors)
                }
            }
        }
    })
}

#[derive(Debug)]
enum RuleAttr {
    Rule {
        name: String,
        args: Vec<(String, String)>,
    },
    Alias(String),
    OmitEmpty,
}

fn parse_rules(attrs: &[syn::Attribute]) -> syn::Result<Vec<RuleAttr>> {
    let mut rules = Vec::new();

    for attr in attrs {
        if !attr.path().is_ident("validate") {
            continue;
        }

        attr.parse_nested_meta(|meta| {
            if meta.path.is_ident("required") {
                rules.push(rule("required", Vec::new()));
                return Ok(());
            }

            if meta.path.is_ident("omitempty") {
                rules.push(RuleAttr::OmitEmpty);
                return Ok(());
            }

            if meta.path.is_ident("alias") {
                let value = meta.value()?;
                let alias: LitStr = value.parse()?;
                rules.push(RuleAttr::Alias(alias.value()));
                return Ok(());
            }

            if meta.path.is_ident("min") {
                let value = meta.value()?;
                rules.push(rule("min", vec![("min".to_owned(), parse_arg_value(value)?)]));
                return Ok(());
            }

            if meta.path.is_ident("max") {
                let value = meta.value()?;
                rules.push(rule("max", vec![("max".to_owned(), parse_arg_value(value)?)]));
                return Ok(());
            }

            if meta.path.is_ident("gt") {
                let value = meta.value()?;
                rules.push(rule("gt", vec![("value".to_owned(), parse_arg_value(value)?)]));
                return Ok(());
            }

            if meta.path.is_ident("gte") {
                let value = meta.value()?;
                rules.push(rule("gte", vec![("value".to_owned(), parse_arg_value(value)?)]));
                return Ok(());
            }

            if meta.path.is_ident("lt") {
                let value = meta.value()?;
                rules.push(rule("lt", vec![("value".to_owned(), parse_arg_value(value)?)]));
                return Ok(());
            }

            if meta.path.is_ident("lte") {
                let value = meta.value()?;
                rules.push(rule("lte", vec![("value".to_owned(), parse_arg_value(value)?)]));
                return Ok(());
            }

            if meta.path.is_ident("length") {
                rules.push(rule(
                    "length",
                    parse_keyed_args(meta, &["min", "max", "exact"])?,
                ));
                return Ok(());
            }

            if meta.path.is_ident("range") {
                rules.push(rule("range", parse_keyed_args(meta, &["min", "max"])?));
                return Ok(());
            }

            if meta.path.is_ident("regex") {
                rules.push(rule("regex", parse_keyed_args(meta, &["pattern"])?));
                return Ok(());
            }

            if meta.path.is_ident("oneof") {
                rules.push(rule(
                    "oneof",
                    vec![("values".to_owned(), parse_oneof_values(meta)?)],
                ));
                return Ok(());
            }

            for name in MARKER_RULES {
                if meta.path.is_ident(*name) {
                    rules.push(rule(*name, Vec::new()));
                    return Ok(());
                }
            }

            for name in STRING_ARG_RULES {
                if meta.path.is_ident(*name) {
                    rules.push(rule(*name, parse_keyed_args(meta, &["value"])?));
                    return Ok(());
                }
            }

            Err(meta.error("unsupported validate rule"))
        })?;
    }

    Ok(rules)
}

const MARKER_RULES: &[&str] = &[
    "email",
    "url",
    "alpha",
    "alphanum",
    "numeric",
    "number",
    "lowercase",
    "uppercase",
    "boolean",
    "hexcolor",
    "rgb",
    "rgba",
    "hsl",
    "hsla",
    "cmyk",
];

const STRING_ARG_RULES: &[&str] = &["contains", "startswith", "endswith"];

fn rule(name: impl Into<String>, args: Vec<(String, String)>) -> RuleAttr {
    RuleAttr::Rule {
        name: name.into(),
        args,
    }
}

fn parse_keyed_args(
    meta: ParseNestedMeta<'_>,
    allowed: &[&str],
) -> syn::Result<Vec<(String, String)>> {
    let mut args = Vec::new();

    meta.parse_nested_meta(|nested| {
        for name in allowed {
            if nested.path.is_ident(name) {
                args.push(((*name).to_owned(), parse_arg_value(nested.value()?)?));
                return Ok(());
            }
        }

        Err(nested.error("unsupported validate argument"))
    })?;

    Ok(args)
}

fn parse_oneof_values(meta: ParseNestedMeta<'_>) -> syn::Result<String> {
    let content;
    parenthesized!(content in meta.input);

    let mut values = Vec::new();
    while !content.is_empty() {
        let value: LitStr = content.parse()?;
        values.push(value.value());

        if content.peek(Token![,]) {
            content.parse::<Token![,]>()?;
        }
    }

    if values.is_empty() {
        return Err(meta.error("oneof requires at least one value"));
    }

    Ok(values.join(","))
}

fn parse_arg_value(input: syn::parse::ParseStream<'_>) -> syn::Result<String> {
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
                "negative validate argument must be an integer or float literal",
            )),
        },
        expr => Err(syn::Error::new_spanned(
            expr,
            "validate argument must be an integer, float, or string literal",
        )),
    }
}

fn parse_lit(lit: Lit) -> syn::Result<String> {
    match lit {
        Lit::Int(value) => Ok(value.base10_digits().to_owned()),
        Lit::Float(value) => Ok(value.base10_digits().to_owned()),
        Lit::Str(value) => Ok(value.value()),
        _ => Err(syn::Error::new_spanned(
            lit,
            "validate argument must be an integer, float, or string literal",
        )),
    }
}
