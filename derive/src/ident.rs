use syn::ext::IdentExt;

pub(super) fn canonical(ident: &syn::Ident) -> String {
    ident.unraw().to_string()
}

pub(super) fn member_name(name: &str) -> Option<syn::Ident> {
    syn::parse_str::<syn::Ident>(name)
        .or_else(|_| syn::parse_str::<syn::Ident>(&format!("r#{name}")))
        .ok()
}
