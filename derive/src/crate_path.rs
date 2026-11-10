use quote::quote;

pub(super) fn resolve() -> syn::Result<proc_macro2::TokenStream> {
    // `halo` is the stable public crate name exposed by halo-validator.
    Ok(quote!(::halo))
}
