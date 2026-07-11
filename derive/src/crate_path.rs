use proc_macro_crate::{FoundCrate, crate_name};
use quote::quote;

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
