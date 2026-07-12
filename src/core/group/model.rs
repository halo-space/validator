use std::sync::Arc;

use crate::core::{Access, Context, Items, Kind, Namespace, Params, Rule, Value};
use crate::target::{FieldTarget, namespace_for};

pub(super) struct Execution<'a, 'b, 'c> {
    pub(super) context: &'a Context<'c>,
    pub(super) display_rule: Option<&'a str>,
    pub(super) scope: Scope<'b>,
}

pub(super) struct FieldMeta<'a> {
    pub(super) target: FieldTarget<'a>,
    pub(super) namespace: Namespace,
    pub(super) struct_namespace: Namespace,
}

impl<'a> FieldMeta<'a> {
    pub(super) fn new(target: FieldTarget<'a>) -> Self {
        let namespace = Namespace::new(namespace_for(&target.type_name, &target.field_name));
        let struct_namespace =
            Namespace::new(namespace_for(&target.type_name, &target.struct_field_name));
        Self {
            target,
            namespace,
            struct_namespace,
        }
    }
}

#[derive(Clone, Copy, Default)]
pub(super) struct Scope<'a> {
    pub(super) access: Option<&'a dyn Access>,
    pub(super) items: Option<&'a dyn Items>,
}

#[derive(Clone)]
pub(crate) struct Group {
    pub(super) steps: Vec<Step>,
}

#[derive(Clone)]
pub(super) enum Step {
    Check(Check),
    Any { checks: Vec<Check>, reason: String },
}

#[derive(Clone)]
pub(super) enum Check {
    Rule {
        name: String,
        params: Params,
        handler: Arc<dyn Rule>,
    },
    Alias {
        name: String,
        group: Arc<Group>,
    },
    OmitEmpty,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum Mode {
    Value,
    Fields,
    FieldsWithAliases,
    FieldsAndItems,
    FieldsAndItemsWithAliases,
}

impl Mode {
    pub(super) const fn fields(self) -> bool {
        !matches!(self, Self::Value)
    }
    pub(super) const fn items(self) -> bool {
        matches!(self, Self::FieldsAndItems | Self::FieldsAndItemsWithAliases)
    }
    pub(super) const fn alias(self) -> Self {
        match self {
            Self::FieldsWithAliases | Self::FieldsAndItemsWithAliases => self,
            Self::Value | Self::Fields | Self::FieldsAndItems => Self::Value,
        }
    }
}

#[derive(Clone, Copy, Eq, PartialEq)]
pub(crate) enum Flow {
    Continue,
    Stop,
}

pub(super) enum CheckOutput {
    Pass,
    Fail,
    Stop,
}

pub(super) struct TypeValue {
    pub(super) kind: Kind,
}

impl Value for TypeValue {
    fn kind(&self) -> Kind {
        self.kind
    }
    fn required(&self) -> bool {
        false
    }
}
