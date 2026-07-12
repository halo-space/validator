use std::sync::{Arc, RwLock, RwLockReadGuard, RwLockWriteGuard};

use crate::Error;
use crate::core::{Expr, Group, Spec, parse_expression};
use crate::schema::{Schema, Tree};

use super::Validator;

trait RwLockExt<T> {
    fn read_unpoisoned(&self) -> RwLockReadGuard<'_, T>;
    fn write_unpoisoned(&self) -> RwLockWriteGuard<'_, T>;
}

impl<T> RwLockExt<T> for RwLock<T> {
    fn read_unpoisoned(&self) -> RwLockReadGuard<'_, T> {
        self.read().unwrap_or_else(|error| error.into_inner())
    }

    fn write_unpoisoned(&self) -> RwLockWriteGuard<'_, T> {
        self.write().unwrap_or_else(|error| error.into_inner())
    }
}

#[derive(Clone, Eq, Ord, PartialEq, PartialOrd)]
pub(super) struct SpecKey {
    spec: Spec,
    items: bool,
}

impl Validator {
    fn parse(&self, expression: &str) -> Result<Arc<Vec<Expr>>, Error> {
        if let Some(exprs) = self
            .expression_cache
            .read_unpoisoned()
            .get(expression)
            .cloned()
        {
            return Ok(exprs);
        }

        let exprs = Arc::new(parse_expression(expression)?);
        let mut cache = self.expression_cache.write_unpoisoned();
        if let Some(exprs) = cache.get(expression).cloned() {
            return Ok(exprs);
        }

        cache.insert(expression.to_owned(), exprs.clone());
        Ok(exprs)
    }

    pub(super) fn compile(&self, expression: &str) -> Result<Arc<Group>, Error> {
        if let Some(group) = self
            .compiled_cache
            .read_unpoisoned()
            .get(expression)
            .cloned()
        {
            return Ok(group);
        }

        let exprs = self.parse(expression)?;
        let group = Arc::new(Group::compile(exprs.as_ref(), &self.registry)?);
        let mut cache = self.compiled_cache.write_unpoisoned();
        if let Some(group) = cache.get(expression).cloned() {
            return Ok(group);
        }

        cache.insert(expression.to_owned(), group.clone());
        Ok(group)
    }

    pub(super) fn schema_tree(&self, schema: &Schema) -> Result<Arc<Tree>, Error> {
        let key = schema.id();
        if let Some(tree) = self.schema_cache.read_unpoisoned().get(&key).cloned() {
            return Ok(tree);
        }

        let tree = Arc::new(schema.compile(&self.registry)?);
        let mut cache = self.schema_cache.write_unpoisoned();
        if let Some(tree) = cache.get(&key).cloned() {
            return Ok(tree);
        }

        cache.insert(key, tree.clone());
        Ok(tree)
    }

    pub(super) fn compile_spec(&self, spec: Spec, items: bool) -> Result<Arc<Group>, Error> {
        let key = SpecKey { spec, items };
        if let Some(group) = self.spec_cache.read_unpoisoned().get(&key).cloned() {
            return Ok(group);
        }

        let group = Arc::new(if items {
            Group::compile_spec_with_items(&key.spec, &self.registry)?
        } else {
            Group::compile_spec(&key.spec, &self.registry)?
        });
        let mut cache = self.spec_cache.write_unpoisoned();
        if let Some(group) = cache.get(&key).cloned() {
            return Ok(group);
        }

        cache.insert(key, group.clone());
        Ok(group)
    }
}
