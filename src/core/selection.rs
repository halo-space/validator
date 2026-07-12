use std::cell::Cell;

use super::{Error, Namespace};

pub(crate) struct Fields {
    values: Vec<Field>,
}

struct Field {
    path: String,
    matched: Cell<bool>,
}

impl Fields {
    pub(crate) fn new<I, S>(fields: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: AsRef<str>,
    {
        Self {
            values: fields
                .into_iter()
                .map(|field| Field {
                    path: field.as_ref().to_owned(),
                    matched: Cell::new(false),
                })
                .collect(),
        }
    }

    pub(crate) fn verify(&self) -> Result<(), Error> {
        match self.values.iter().find(|field| !field.matched.get()) {
            Some(field) => Err(Error::UnknownField {
                field: field.path.clone(),
            }),
            None => Ok(()),
        }
    }

    fn partial(&self, path: &str) -> bool {
        self.visit(path);
        self.values.iter().any(|field| related(path, &field.path))
    }

    fn except(&self, path: &str) -> bool {
        self.visit(path);
        !self
            .values
            .iter()
            .any(|field| path == field.path || descendant(path, &field.path))
    }

    fn active(&self, path: &str, partial: bool) -> bool {
        if path.is_empty() {
            return !partial || !self.values.is_empty();
        }

        if partial {
            self.values.iter().any(|field| related(path, &field.path))
        } else {
            !self
                .values
                .iter()
                .any(|field| path == field.path || descendant(path, &field.path))
        }
    }

    fn visit(&self, path: &str) {
        for field in &self.values {
            if path == field.path || descendant(path, &field.path) {
                field.matched.set(true);
            }
        }
    }
}

#[derive(Clone, Copy)]
pub(crate) enum Selection<'a> {
    Full,
    Partial(&'a Fields),
    Except(&'a Fields),
    Filter(&'a dyn Fn(&Namespace) -> bool),
}

impl Selection<'_> {
    pub(crate) fn includes(&self, path: &str) -> bool {
        match self {
            Self::Full => true,
            Self::Partial(fields) => fields.partial(path),
            Self::Except(fields) => fields.except(path),
            Self::Filter(filter) => {
                ancestors(path).all(|path| filter(&Namespace::new(path.to_owned())))
            }
        }
    }

    pub(crate) fn active(&self, path: &str) -> bool {
        match self {
            Self::Full => true,
            Self::Partial(fields) => fields.active(path, true),
            Self::Except(fields) => fields.active(path, false),
            Self::Filter(filter) => {
                path.is_empty()
                    || ancestors(path).all(|path| filter(&Namespace::new(path.to_owned())))
            }
        }
    }

    pub(crate) fn is_full(&self) -> bool {
        matches!(self, Self::Full)
    }
}

fn related(left: &str, right: &str) -> bool {
    left == right || descendant(left, right) || descendant(right, left)
}

fn descendant(path: &str, parent: &str) -> bool {
    path.strip_prefix(parent)
        .is_some_and(|rest| rest.starts_with('.') || rest.starts_with('['))
}

fn ancestors(path: &str) -> impl Iterator<Item = &str> {
    let mut boundaries = Vec::new();
    let mut quote = false;
    let mut escaped = false;
    let mut bracket = false;

    for (index, ch) in path.char_indices() {
        if bracket {
            if quote {
                if escaped {
                    escaped = false;
                } else if ch == '\\' {
                    escaped = true;
                } else if ch == '"' {
                    quote = false;
                }
            } else if ch == '"' {
                quote = true;
            } else if ch == ']' {
                bracket = false;
                boundaries.push(index + ch.len_utf8());
            }
            continue;
        }

        match ch {
            '.' => boundaries.push(index),
            '[' => {
                boundaries.push(index);
                bracket = true;
            }
            _ => {}
        }
    }

    boundaries.push(path.len());
    boundaries.dedup();
    boundaries
        .into_iter()
        .filter_map(|end| if end == 0 { None } else { Some(&path[..end]) })
}

#[cfg(test)]
mod tests {
    use super::{Fields, Selection, ancestors};

    #[test]
    fn selected_fields_match_only_boundaries() {
        let fields = Fields::new(["profile.email"]);
        let selection = Selection::Partial(&fields);

        assert!(selection.includes("profile"));
        assert!(selection.includes("profile.email"));
        assert!(!selection.includes("profiled"));
        assert!(!selection.includes("profile.email_address"));
        assert!(fields.verify().is_ok());
    }

    #[test]
    fn selected_collection_index_is_not_matched_by_its_parent() {
        let fields = Fields::new(["items[0].email"]);
        let selection = Selection::Partial(&fields);

        assert!(selection.includes("items"));
        assert!(fields.verify().is_err());
        assert!(selection.includes("items[0].email"));
        assert!(fields.verify().is_ok());
    }

    #[test]
    fn path_ancestors_ignore_separators_inside_map_keys() {
        assert_eq!(
            ancestors(r#"values["a.b"].email"#).collect::<Vec<_>>(),
            ["values", r#"values["a.b"]"#, r#"values["a.b"].email"#]
        );
    }
}
