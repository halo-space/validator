use std::collections::{BTreeMap, HashMap};
use std::hash::Hash;

use super::{Kind, Value};

impl<T: Value> Value for Vec<T> {
    fn kind(&self) -> Kind {
        Kind::Vec
    }

    fn declared_kind() -> Option<Kind> {
        Some(Kind::Vec)
    }

    fn required(&self) -> bool {
        !Vec::is_empty(self)
    }

    fn len(&self) -> Option<usize> {
        Some(self.len())
    }

    fn array_items(&self) -> Option<Box<dyn Iterator<Item = &dyn Value> + '_>> {
        Some(Box::new(self.iter().map(|item| item as &dyn Value)))
    }
}

impl<T: Value, const N: usize> Value for [T; N] {
    fn kind(&self) -> Kind {
        Kind::Array
    }

    fn declared_kind() -> Option<Kind> {
        Some(Kind::Array)
    }

    fn required(&self) -> bool {
        N > 0
    }

    fn len(&self) -> Option<usize> {
        Some(N)
    }

    fn array_items(&self) -> Option<Box<dyn Iterator<Item = &dyn Value> + '_>> {
        Some(Box::new(self.iter().map(|item| item as &dyn Value)))
    }
}

impl<T: Value> Value for [T] {
    fn kind(&self) -> Kind {
        Kind::Slice
    }

    fn required(&self) -> bool {
        !self.is_empty()
    }

    fn len(&self) -> Option<usize> {
        Some(self.len())
    }

    fn array_items(&self) -> Option<Box<dyn Iterator<Item = &dyn Value> + '_>> {
        Some(Box::new(self.iter().map(|item| item as &dyn Value)))
    }
}

impl<T: Value> Value for &[T] {
    fn kind(&self) -> Kind {
        Kind::Slice
    }

    fn declared_kind() -> Option<Kind> {
        Some(Kind::Slice)
    }

    fn required(&self) -> bool {
        !<[T]>::is_empty(self)
    }

    fn len(&self) -> Option<usize> {
        Some(<[T]>::len(self))
    }

    fn array_items(&self) -> Option<Box<dyn Iterator<Item = &dyn Value> + '_>> {
        Some(Box::new(self.iter().map(|item| item as &dyn Value)))
    }
}

impl<K, V: Value> Value for BTreeMap<K, V> {
    fn kind(&self) -> Kind {
        Kind::Map
    }

    fn declared_kind() -> Option<Kind> {
        Some(Kind::Map)
    }

    fn required(&self) -> bool {
        !self.is_empty()
    }

    fn len(&self) -> Option<usize> {
        Some(self.len())
    }

    fn map_values(&self) -> Option<Box<dyn Iterator<Item = &dyn Value> + '_>> {
        Some(Box::new(self.values().map(|value| value as &dyn Value)))
    }
}

impl<K: Eq + Hash, V: Value> Value for HashMap<K, V> {
    fn kind(&self) -> Kind {
        Kind::Map
    }

    fn declared_kind() -> Option<Kind> {
        Some(Kind::Map)
    }

    fn required(&self) -> bool {
        !self.is_empty()
    }

    fn len(&self) -> Option<usize> {
        Some(self.len())
    }

    fn map_values(&self) -> Option<Box<dyn Iterator<Item = &dyn Value> + '_>> {
        Some(Box::new(self.values().map(|value| value as &dyn Value)))
    }
}
