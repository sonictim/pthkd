//! Parameter helper for action functions
//!
//! Provides a clean API for accessing typed parameters from config.toml

use std::collections::HashMap;
use std::sync::Arc;
use toml::Value;

/// Wrapper for action parameters with type-safe accessor methods
/// Uses Arc for cheap cloning when passing to async actions
#[derive(Debug, Clone)]
pub struct Params(Arc<HashMap<String, Value>>);

impl Params {
    /// Create a new Params from a HashMap
    pub fn new(map: HashMap<String, Value>) -> Self {
        Params(Arc::new(map))
    }

    /// Create an empty Params (for actions with no parameters)
    pub fn empty() -> Self {
        Params(Arc::new(HashMap::new()))
    }

    /// Get a string parameter as a borrowed &str with a default value
    ///
    /// Use this when you don't need ownership of the string (avoids allocation).
    /// The returned reference is valid for the lifetime of the Params or the default parameter.
    ///
    /// # Example
    /// ```ignore
    /// let direction = params.get_str("direction", "next");
    /// ```
    pub fn get_str<'a>(&'a self, key: &str, default: &'a str) -> &'a str {
        self.0
            .get(key)
            .and_then(|v| v.as_str())
            .unwrap_or(default)
    }

    /// Get a string parameter with a default value
    ///
    /// Returns an owned String. Use get_str() if you don't need ownership.
    ///
    /// # Example
    /// ```ignore
    /// let direction = params.get_string("direction", "next");
    /// ```
    pub fn get_string(&self, key: &str, default: &str) -> String {
        self.get_str(key, default).to_string()
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

    /// Get timeout in milliseconds with a default value
    ///
    /// Convenience method for getting timeout parameters as u64.
    ///
    /// # Example
    /// ```ignore
    /// let timeout_ms = params.get_timeout_ms("timeout", 500);
    /// ```
    pub fn get_timeout_ms(&self, key: &str, default: u64) -> u64 {
        self.get_int(key, default as i64) as u64
    }

    /// Get a simple array of strings
    ///
    /// # Example
    /// ```ignore
    /// // In config.toml:
    /// // plugins = ["Reverse", "Normalize", "Gain"]
    /// let plugins = params.get_string_vec("plugins");
    /// // Returns: vec!["Reverse", "Normalize", "Gain"]
    /// ```
    pub fn get_string_vec(&self, key: &str) -> Vec<String> {
        self.0
            .get(key)
            .and_then(|v| v.as_array())
            .map(|array| {
                array
                    .iter()
                    .filter_map(|s| s.as_str().map(|s| s.to_string()))
                    .collect()
            })
            .unwrap_or_default()
    }

    /// Get a nested array of strings (array of arrays)
    ///
    /// # Example
    /// ```ignore
    /// // In config.toml:
    /// // plugins = [["Other", "Reverse"], ["Valhalla DSP", "Vintage verb"]]
    /// let plugins = params.get_nested_strings("plugins");
    /// // Returns: vec![vec!["Other", "Reverse"], vec!["Valhalla DSP", "Vintage verb"]]
    /// ```
    pub fn get_nested_strings(&self, key: &str) -> Vec<Vec<String>> {
        self.0
            .get(key)
            .and_then(|v| v.as_array())
            .map(|outer_array| {
                outer_array
                    .iter()
                    .filter_map(|inner_value| {
                        inner_value.as_array().map(|inner_array| {
                            inner_array
                                .iter()
                                .filter_map(|s| s.as_str().map(|s| s.to_string()))
                                .collect()
                        })
                    })
                    .collect()
            })
            .unwrap_or_default()
    }

    /// Get a nested array of strings as pairs (tuples)
    ///
    /// Convenience method for when you know each inner array has exactly 2 elements.
    /// If an inner array doesn't have exactly 2 elements, it's skipped.
    ///
    /// # Example
    /// ```ignore
    /// // In config.toml:
    /// // plugins = [["Other", "Reverse"], ["Valhalla DSP", "Vintage verb"]]
    /// let plugins = params.get_string_pairs("plugins");
    /// // Returns: vec![("Other", "Reverse"), ("Valhalla DSP", "Vintage verb")]
    /// ```
    pub fn get_string_pairs(&self, key: &str) -> Vec<(String, String)> {
        self.get_nested_strings(key)
            .into_iter()
            .filter_map(|inner| {
                if inner.len() == 2 {
                    Some((inner[0].clone(), inner[1].clone()))
                } else {
                    None
                }
            })
            .collect()
    }
}

impl Default for Params {
    fn default() -> Self {
        Self::empty()
    }
}

impl From<HashMap<String, Value>> for Params {
    fn from(map: HashMap<String, Value>) -> Self {
        Params(Arc::new(map))
    }
}
