use proc_macro_crate::{FoundCrate, crate_name};
use quote::quote;
use syn::ext::IdentExt;

pub(super) fn validator_crate_path() -> syn::Result<proc_macro2::TokenStream> {
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

pub(super) fn canonical(ident: &syn::Ident) -> String {
    ident.unraw().to_string()
}

pub(super) fn member_name(name: &str) -> Option<syn::Ident> {
    syn::parse_str::<syn::Ident>(name)
        .or_else(|_| syn::parse_str::<syn::Ident>(&format!("r#{name}")))
        .ok()
}

pub(super) const CONDITIONAL_PAIR_RULES: &[&str] = &[
    "required_if",
    "required_unless",
    "skip_unless",
    "excluded_if",
    "excluded_unless",
];

pub(super) const CONDITIONAL_FIELD_LIST_RULES: &[&str] = &[
    "required_with",
    "required_with_all",
    "required_without",
    "required_without_all",
    "excluded_with",
    "excluded_with_all",
    "excluded_without",
    "excluded_without_all",
];
