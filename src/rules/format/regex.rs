use std::collections::BTreeMap;
use std::sync::{Arc, RwLock};

use regex::Regex;

use crate::{Field, Rule};

#[derive(Debug, Default)]
pub struct RegexRule {
    cache: RwLock<BTreeMap<String, Option<Arc<Regex>>>>,
}

impl Rule for RegexRule {
    fn check(&self, field: &Field<'_>) -> Result<bool, crate::Error> {
        let Some(pattern) = field.params().get("pattern") else {
            return Ok(false);
        };
        let Some(value) = field.value().string() else {
            return Ok(false);
        };

        Ok(self
            .regex(pattern)
            .is_some_and(|regex| regex.is_match(value.as_ref())))
    }
}

impl RegexRule {
    fn regex(&self, pattern: &str) -> Option<Arc<Regex>> {
        if let Some(regex) = self
            .cache
            .read()
            .expect("regex cache lock must not be poisoned")
            .get(pattern)
            .cloned()
        {
            return regex;
        }

        let regex = Regex::new(pattern).ok().map(Arc::new);
        let mut cache = self
            .cache
            .write()
            .expect("regex cache lock must not be poisoned");
        if let Some(regex) = cache.get(pattern).cloned() {
            return regex;
        }

        cache.insert(pattern.to_owned(), regex.clone());
        regex
    }
}
