use std::collections::BTreeMap;
use std::sync::{Arc, RwLock};

use regex::Regex as Pattern;

use crate::{Field, Rule};

#[derive(Debug, Default)]
pub struct Regex {
    cache: RwLock<BTreeMap<String, Option<Arc<Pattern>>>>,
}

impl Rule for Regex {
    fn check(&self, field: &Field<'_>) -> Result<bool, crate::Error> {
        let Some(pattern) = field.params().get("pattern") else {
            return Ok(false);
        };
        let Some(value) = field.value().string() else {
            return Ok(false);
        };

        Ok(self
            .pattern(pattern)
            .is_some_and(|regex| regex.is_match(value.as_ref())))
    }
}

impl Regex {
    fn pattern(&self, pattern: &str) -> Option<Arc<Pattern>> {
        if let Some(regex) = self
            .cache
            .read()
            .expect("regex cache lock must not be poisoned")
            .get(pattern)
            .cloned()
        {
            return regex;
        }

        let regex = Pattern::new(pattern).ok().map(Arc::new);
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
