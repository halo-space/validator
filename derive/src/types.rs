use proc_macro2::TokenStream;
use quote::quote;
use syn::{GenericArgument, PathArguments, Type, TypeArray, TypeReference};

pub(crate) fn is_option(ty: &Type) -> bool {
    let Type::Path(path) = ty else {
        return false;
    };

    path.path.segments.last().is_some_and(|segment| {
        segment.ident == "Option" && matches!(segment.arguments, PathArguments::AngleBracketed(_))
    })
}

pub(crate) fn collection_item(ty: &Type) -> Option<&Type> {
    match ty {
        Type::Path(path) => path.path.segments.last().and_then(|segment| {
            (segment.ident == "Vec")
                .then(|| first_type_argument(segment))
                .flatten()
        }),
        Type::Array(TypeArray { elem, .. }) => Some(elem.as_ref()),
        Type::Reference(TypeReference { elem, .. }) => match elem.as_ref() {
            Type::Slice(slice) => Some(slice.elem.as_ref()),
            _ => None,
        },
        _ => None,
    }
}

pub(crate) fn map_key(ty: &Type) -> Option<&Type> {
    map_type(ty, 0)
}

pub(crate) fn map_value(ty: &Type) -> Option<&Type> {
    map_type(ty, 1)
}

fn map_type(ty: &Type, index: usize) -> Option<&Type> {
    let Type::Path(path) = ty else {
        return None;
    };
    let segment = path.path.segments.last()?;
    if segment.ident != "HashMap" && segment.ident != "BTreeMap" {
        return None;
    }
    let PathArguments::AngleBracketed(arguments) = &segment.arguments else {
        return None;
    };

    arguments
        .args
        .iter()
        .filter_map(|argument| match argument {
            GenericArgument::Type(ty) => Some(ty),
            _ => None,
        })
        .nth(index)
}

pub(crate) fn collection_kind(ty: &Type, crate_path: &TokenStream) -> Option<TokenStream> {
    match ty {
        Type::Path(path)
            if path
                .path
                .segments
                .last()
                .is_some_and(|segment| segment.ident == "Vec") =>
        {
            Some(quote!(#crate_path::Kind::Vec))
        }
        Type::Array(_) => Some(quote!(#crate_path::Kind::Array)),
        Type::Reference(TypeReference { elem, .. }) if matches!(elem.as_ref(), Type::Slice(_)) => {
            Some(quote!(#crate_path::Kind::Slice))
        }
        _ => None,
    }
}

pub(crate) fn field_kind(ty: &Type, value: TokenStream, crate_path: &TokenStream) -> TokenStream {
    if let Type::Path(path) = ty
        && let Some(segment) = path.path.segments.last()
        && segment.ident == "Option"
        && let Some(inner) = first_type_argument(segment)
    {
        let inner = field_kind(inner, quote!(value), crate_path);
        return quote! {
            match #value {
                Some(value) => #inner,
                None => #crate_path::Kind::Option,
            }
        };
    }

    declared_kind(ty, crate_path)
}

fn declared_kind(ty: &Type, crate_path: &TokenStream) -> TokenStream {
    match ty {
        Type::Array(_) => quote!(#crate_path::Kind::Array),
        Type::Slice(_) => quote!(#crate_path::Kind::Slice),
        Type::Reference(reference) => declared_kind(reference.elem.as_ref(), crate_path),
        Type::Path(path) => {
            let Some(segment) = path.path.segments.last() else {
                return quote!(#crate_path::Kind::Other);
            };

            match segment.ident.to_string().as_str() {
                "Option" => first_type_argument(segment)
                    .map(|ty| declared_kind(ty, crate_path))
                    .unwrap_or_else(|| quote!(#crate_path::Kind::Option)),
                "String" | "str" => quote!(#crate_path::Kind::String),
                "bool" => quote!(#crate_path::Kind::Bool),
                "i8" => quote!(#crate_path::Kind::Int(#crate_path::IntKind::I8)),
                "i16" => quote!(#crate_path::Kind::Int(#crate_path::IntKind::I16)),
                "i32" => quote!(#crate_path::Kind::Int(#crate_path::IntKind::I32)),
                "i64" => quote!(#crate_path::Kind::Int(#crate_path::IntKind::I64)),
                "i128" => quote!(#crate_path::Kind::Int(#crate_path::IntKind::I128)),
                "isize" => quote!(#crate_path::Kind::Int(#crate_path::IntKind::Isize)),
                "u8" => quote!(#crate_path::Kind::Uint(#crate_path::UintKind::U8)),
                "u16" => quote!(#crate_path::Kind::Uint(#crate_path::UintKind::U16)),
                "u32" => quote!(#crate_path::Kind::Uint(#crate_path::UintKind::U32)),
                "u64" => quote!(#crate_path::Kind::Uint(#crate_path::UintKind::U64)),
                "u128" => quote!(#crate_path::Kind::Uint(#crate_path::UintKind::U128)),
                "usize" => quote!(#crate_path::Kind::Uint(#crate_path::UintKind::Usize)),
                "f32" => quote!(#crate_path::Kind::Float(#crate_path::FloatKind::F32)),
                "f64" => quote!(#crate_path::Kind::Float(#crate_path::FloatKind::F64)),
                "Vec" => quote!(#crate_path::Kind::Vec),
                "BTreeMap" | "HashMap" => quote!(#crate_path::Kind::Map),
                "SystemTime" => quote!(#crate_path::Kind::Time),
                _ => quote!(#crate_path::Kind::Other),
            }
        }
        _ => quote!(#crate_path::Kind::Other),
    }
}

fn first_type_argument(segment: &syn::PathSegment) -> Option<&Type> {
    let PathArguments::AngleBracketed(arguments) = &segment.arguments else {
        return None;
    };

    arguments.args.iter().find_map(|argument| match argument {
        GenericArgument::Type(ty) => Some(ty),
        _ => None,
    })
}
