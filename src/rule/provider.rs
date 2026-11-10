use crate::Error;
use crate::validator::Config;

/// Registers a reusable group of rules and aliases into validator configuration.
pub trait Provider {
    /// Extends the supplied configuration and returns the updated configuration.
    fn register(&self, config: Config) -> Result<Config, Error>;
}
