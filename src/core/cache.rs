use std::borrow::Borrow;
use std::collections::{BTreeMap, VecDeque};

pub(crate) const CAPACITY: usize = 256;

#[derive(Debug)]
pub(crate) struct Cache<K, V> {
    capacity: usize,
    order: VecDeque<K>,
    values: BTreeMap<K, V>,
}

impl<K, V> Cache<K, V>
where
    K: Clone + Ord,
{
    pub(crate) fn new(capacity: usize) -> Self {
        Self {
            capacity,
            order: VecDeque::new(),
            values: BTreeMap::new(),
        }
    }

    pub(crate) fn get<Q>(&mut self, key: &Q) -> Option<&V>
    where
        K: Borrow<Q>,
        Q: Ord + ?Sized,
    {
        if self.values.contains_key(key)
            && let Some(index) = self
                .order
                .iter()
                .position(|candidate| candidate.borrow() == key)
            && let Some(touched) = self.order.remove(index)
        {
            self.order.push_back(touched);
        }
        self.values.get(key)
    }

    pub(crate) fn insert(&mut self, key: K, value: V) {
        if self.capacity == 0 {
            return;
        }

        if let Some(current) = self.values.get_mut(&key) {
            *current = value;
            if let Some(index) = self.order.iter().position(|candidate| candidate == &key)
                && let Some(touched) = self.order.remove(index)
            {
                self.order.push_back(touched);
            }
            return;
        }

        if self.values.len() == self.capacity
            && let Some(oldest) = self.order.pop_front()
        {
            self.values.remove(&oldest);
        }

        self.order.push_back(key.clone());
        self.values.insert(key, value);
    }

    #[cfg(test)]
    pub(crate) fn len(&self) -> usize {
        self.values.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn evicts_least_recently_used_entry_at_capacity() {
        let mut cache = Cache::new(2);
        cache.insert("first", 1);
        cache.insert("second", 2);
        assert_eq!(cache.get(&"first"), Some(&1));
        cache.insert("third", 3);

        assert_eq!(cache.len(), 2);
        assert_eq!(cache.get(&"first"), Some(&1));
        assert_eq!(cache.get(&"second"), None);
        assert_eq!(cache.get(&"third"), Some(&3));
    }
}
