use std::sync::{Arc, RwLock};

use regex::Regex as Pattern;

use crate::core::{CAPACITY, Cache};
use crate::{Field, Rule, Signature};

#[derive(Debug)]
pub struct Regex {
    cache: RwLock<Cache<String, Result<Arc<Pattern>, String>>>,
}

impl Default for Regex {
    fn default() -> Self {
        Self {
            cache: RwLock::new(Cache::new(CAPACITY)),
        }
    }
}

impl Rule for Regex {
    fn signature(&self) -> Signature {
        Signature::text("pattern")
    }

    fn validate_params(&self, field: &Field<'_>) -> Result<(), crate::Error> {
        let pattern =
            field
                .params()
                .text("pattern")
                .ok_or_else(|| crate::Error::InvalidRuleExpression {
                    expression: "regex".to_owned(),
                    reason: "rule requires exactly one 'pattern' parameter".to_owned(),
                })?;
        self.pattern(pattern).map(|_| ())
    }

    fn check(&self, field: &Field<'_>) -> Result<bool, crate::Error> {
        let Some(pattern) = field.params().text("pattern") else {
            return Ok(false);
        };
        let Some(value) = field.value().string() else {
            return Ok(false);
        };

        Ok(self.pattern(pattern)?.is_match(value.as_ref()))
    }
}

impl Regex {
    fn pattern(&self, pattern: &str) -> Result<Arc<Pattern>, crate::Error> {
        if let Some(regex) = self
            .cache
            .read()
            .expect("regex cache lock must not be poisoned")
            .get(pattern)
            .cloned()
        {
            return regex.map_err(|reason| invalid(pattern, reason));
        }

        let regex = Pattern::new(pattern)
            .map(Arc::new)
            .map_err(|error| error.to_string());
        let mut cache = self
            .cache
            .write()
            .expect("regex cache lock must not be poisoned");
        if let Some(regex) = cache.get(pattern).cloned() {
            return regex.map_err(|reason| invalid(pattern, reason));
        }

        cache.insert(pattern.to_owned(), regex.clone());
        regex.map_err(|reason| invalid(pattern, reason))
    }
}

fn invalid(pattern: &str, reason: String) -> crate::Error {
    crate::Error::InvalidRuleExpression {
        expression: format!("regex(pattern={pattern:?})"),
        reason: format!("invalid regex pattern: {reason}"),
    }
}
