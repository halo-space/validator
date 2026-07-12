#![allow(missing_docs)]

use std::time::SystemTime;

use super::{Fields, Namespace, Selection};

#[doc(hidden)]
#[derive(Clone)]
pub struct Context<'a> {
    now: SystemTime,
    selection: Selection<'a>,
    prefix: String,
}

impl Context<'_> {
    pub fn new() -> Self {
        Self {
            now: SystemTime::now(),
            selection: Selection::Full,
            prefix: String::new(),
        }
    }

    pub(crate) fn partial(fields: &Fields) -> Context<'_> {
        Context {
            now: SystemTime::now(),
            selection: Selection::Partial(fields),
            prefix: String::new(),
        }
    }

    pub(crate) fn except(fields: &Fields) -> Context<'_> {
        Context {
            now: SystemTime::now(),
            selection: Selection::Except(fields),
            prefix: String::new(),
        }
    }

    pub fn filter(filter: &dyn Fn(&Namespace) -> bool) -> Context<'_> {
        Context {
            now: SystemTime::now(),
            selection: Selection::Filter(filter),
            prefix: String::new(),
        }
    }

    pub fn includes(&self, field: &str) -> bool {
        if self.selection.is_full() {
            return true;
        }
        self.selection.includes(&self.path(field))
    }

    pub fn active(&self) -> bool {
        self.selection.active(&self.prefix)
    }

    pub fn is_full(&self) -> bool {
        self.selection.is_full()
    }

    pub fn child(&self, field: &str) -> Self {
        Self {
            now: self.now,
            selection: self.selection,
            prefix: match self.selection {
                Selection::Full => String::new(),
                Selection::Partial(_) | Selection::Except(_) | Selection::Filter(_) => {
                    self.path(field)
                }
            },
        }
    }

    pub fn now(&self) -> SystemTime {
        self.now
    }

    fn path(&self, field: &str) -> String {
        if self.prefix.is_empty() {
            field.to_owned()
        } else if field.starts_with('[') {
            format!("{}{field}", self.prefix)
        } else {
            format!("{}.{field}", self.prefix)
        }
    }
}

impl Default for Context<'_> {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::{Context, Fields};

    #[test]
    fn partial_matches_only_field_boundaries() {
        let fields = Fields::new(["profile.email"]);
        let context = Context::partial(&fields);

        assert!(context.includes("profile"));
        assert!(context.includes("profile.email"));
        assert!(!context.includes("profiled"));
        assert!(!context.includes("profile.email_address"));
    }

    #[test]
    fn child_context_matches_collection_indices() {
        let fields = Fields::new(["items[0].email"]);
        let context = Context::partial(&fields);
        let item = context.child("items[0]");

        assert!(item.active());
        assert!(item.includes("email"));
        assert!(!item.includes("name"));
        assert!(!context.child("items[1]").active());
    }
}
