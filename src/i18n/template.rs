use std::borrow::Cow;
use std::sync::Arc;

use crate::{FieldError, Kind};

use super::translator::Context;

pub type RenderFn = Arc<dyn for<'a> Fn(&Context<'a>) -> String + Send + Sync + 'static>;

#[derive(Clone)]
pub enum Template {
    Text(String),
    Fn(RenderFn),
}

pub(super) fn render(template: &Template, context: &Context<'_>) -> String {
    match template {
        Template::Text(template) => render_text(template, context),
        Template::Fn(render) => render(context),
    }
}

fn render_text(template: &str, context: &Context<'_>) -> String {
    let mut text = String::with_capacity(template.len());
    let mut remaining = template;

    while let Some(start) = remaining.find('{') {
        text.push_str(&remaining[..start]);
        let candidate = &remaining[start + 1..];
        let Some(end) = candidate.find('}') else {
            text.push_str(&remaining[start..]);
            return text;
        };
        let name = &candidate[..end];
        if let Some(value) = placeholder(context, name) {
            text.push_str(&value);
            remaining = &candidate[end + 1..];
        } else {
            text.push('{');
            remaining = candidate;
        }
    }
    text.push_str(remaining);

    text
}

fn placeholder<'a>(context: &'a Context<'_>, name: &str) -> Option<Cow<'a, str>> {
    let value = match name {
        "namespace" => context.namespace().as_str(),
        "struct_namespace" => context.struct_namespace().as_str(),
        "field" => context.field(),
        "struct_field" => context.struct_field(),
        "rule" => context.rule(),
        "reason" => context.reason(),
        "kind" => kind_name(context.kind()),
        _ => {
            return context
                .params()
                .iter()
                .find_map(|(param, value)| (param == name).then(|| Cow::Owned(value.to_string())));
        }
    };
    Some(Cow::Borrowed(value))
}

pub(super) fn default_text(error: &FieldError) -> String {
    format!("{} failed {}", error.namespace().as_str(), error.rule())
}

fn kind_name(kind: Kind) -> &'static str {
    match kind {
        Kind::String => "string",
        Kind::Bool => "bool",
        Kind::Int(_) => "int",
        Kind::Uint(_) => "uint",
        Kind::Float(_) => "float",
        Kind::Vec => "vec",
        Kind::Array => "array",
        Kind::Slice => "slice",
        Kind::Map => "map",
        Kind::Option => "option",
        Kind::Time => "time",
        Kind::Other => "other",
    }
}
