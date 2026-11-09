use quote::quote;

pub(super) fn resolve() -> syn::Result<proc_macro2::TokenStream> {
    Ok(quote!(::halo))
}
