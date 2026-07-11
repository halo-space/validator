#[derive(Clone, Debug)]
pub(super) enum RuleAttr {
    Rule {
        name: String,
        params: Vec<ParamAttr>,
    },
    Alias(String),
    FieldRule {
        name: String,
        params: Vec<ParamAttr>,
    },
    UniqueFields {
        paths: Vec<ItemPath>,
    },
    OmitEmpty,
    Nested,
    Dive(DiveAttr),
}

#[derive(Clone, Debug)]
pub(super) struct ItemPath {
    pub(super) name: String,
    pub(super) segments: Vec<syn::Ident>,
}

#[derive(Clone, Debug)]
pub(super) enum ParamAttr {
    Positional(String),
    Named(String, String),
    List(String, Vec<String>),
}

#[derive(Clone, Debug)]
pub(super) enum DiveAttr {
    Values(Vec<RuleAttr>),
    Map {
        keys: Vec<RuleAttr>,
        values: Vec<RuleAttr>,
    },
}

#[derive(Default)]
pub(super) struct GeneratedChecks {
    pub(super) preflight: Vec<proc_macro2::TokenStream>,
    pub(super) execute: Vec<proc_macro2::TokenStream>,
}
