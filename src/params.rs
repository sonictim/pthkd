//! Parameter helper for action functions
//!
//! Provides a clean API for accessing typed parameters from config.toml

use std::collections::HashMap;
use toml::Value;

/// Wrapper for action parameters with type-safe accessor methods
#[derive(Debug, Clone, Default)]
pub struct Params(HashMap<String, Value>);

impl Params {
    /// Create a new Params from a HashMap
    pub fn new(map: HashMap<String, Value>) -> Self {
        Params(map)
    }

    /// Create an empty Params (for actions with no parameters)
    pub fn empty() -> Self {
        Params(HashMap::new())
    }

    /// Get a string parameter with a default value
    ///
    /// # Example
    /// ```ignore
    /// let direction = params.get_str("direction", "next");
    /// ```
    pub fn get_str(&self, key: &str, default: &str) -> String {
        self.0
            .get(key)
            .and_then(|v| v.as_str())
            .unwrap_or(default)
            .to_string()
    }

    /// Get an integer parameter with a default value
    ///
    /// # Example
    /// ```ignore
    /// let lane = params.get_int("lane", 0);
    /// ```
    pub fn get_int(&self, key: &str, default: i64) -> i64 {
        self.0
            .get(key)
            .and_then(|v| v.as_integer())
            .unwrap_or(default)
    }

    /// Get a boolean parameter with a default value
    ///
    /// # Example
    /// ```ignore
    /// let reverse = params.get_bool("reverse", false);
    /// ```
    pub fn get_bool(&self, key: &str, default: bool) -> bool {
        self.0
            .get(key)
            .and_then(|v| v.as_bool())
            .unwrap_or(default)
    }

    /// Get a float parameter with a default value
    ///
    /// # Example
    /// ```ignore
    /// let speed = params.get_float("speed", 1.0);
    /// ```
    pub fn get_float(&self, key: &str, default: f64) -> f64 {
        self.0
            .get(key)
            .and_then(|v| v.as_float())
            .unwrap_or(default)
    }
}

impl From<HashMap<String, Value>> for Params {
    fn from(map: HashMap<String, Value>) -> Self {
        Params(map)
    }
}
