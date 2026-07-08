use std::time::SystemTime;

#[doc(hidden)]
#[derive(Clone, Debug)]
pub struct Context {
    now: SystemTime,
}

impl Context {
    pub fn new() -> Self {
        Self {
            now: SystemTime::now(),
        }
    }

    pub fn now(&self) -> SystemTime {
        self.now
    }
}

impl Default for Context {
    fn default() -> Self {
        Self::new()
    }
}
