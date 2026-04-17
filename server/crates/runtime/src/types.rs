//! Common types used across all runtime implementations

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use std::str::FromStr;

/// Programming language enum
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Language {
    Python,
    #[serde(rename = "nodejs")]
    NodeJS,
    Java,
    // Future: Go, Ruby, Rust, etc.
}

impl Language {
    /// Get the language name as a string
    pub fn as_str(&self) -> &'static str {
        match self {
            Language::Python => "python",
            Language::NodeJS => "nodejs",
            Language::Java => "java",
        }
    }
}

impl FromStr for Language {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "python" | "python3" | "py" => Ok(Language::Python),
            "nodejs" | "node" | "javascript" | "js" => Ok(Language::NodeJS),
            "java" | "jvm" => Ok(Language::Java),
            _ => Err(format!("Unknown language: {}", s)),
        }
    }
}

impl std::fmt::Display for Language {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

/// Runtime information (interpreter version, path, etc.)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RuntimeInfo {
    /// Programming language
    pub language: Language,

    /// Interpreter/runtime version (e.g., "3.11.5", "20.10.0")
    pub version: String,

    /// Path to the interpreter executable
    pub interpreter_path: PathBuf,

    /// Additional runtime capabilities
    pub capabilities: RuntimeCapabilities,
}

/// Runtime capabilities (what features are supported)
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct RuntimeCapabilities {
    /// Supports warm starts (process reuse)
    pub warm_starts: bool,

    /// Supports async execution
    pub async_execution: bool,

    /// Supports streaming responses
    pub streaming: bool,

    /// Maximum memory limit in MB (None = unlimited)
    pub max_memory_mb: Option<u64>,

    /// Maximum timeout in seconds (None = unlimited)
    pub max_timeout_seconds: Option<u64>,
}

/// Result of a single function invocation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InvocationResult {
    /// Whether the invocation was successful
    pub success: bool,

    /// Function output (JSON string)
    pub output: String,

    /// Error message if failed
    pub error: Option<String>,

    /// Execution time in milliseconds
    pub execution_ms: u64,
}

/// Generic function configuration (language-agnostic)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GenericFunctionConfig {
    /// Unique function name/identifier
    pub name: String,

    /// Programming language
    pub language: Language,

    /// Function source code
    pub code: String,

    /// Entry point/handler name
    pub handler: String,

    /// Environment variables
    #[serde(default)]
    pub environment: HashMap<String, String>,

    /// Memory limit in MB
    #[serde(default = "default_memory_limit")]
    pub memory_limit_mb: u64,

    /// Execution timeout in seconds
    #[serde(default = "default_timeout")]
    pub timeout_seconds: u64,

    /// Working directory (optional)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub working_dir: Option<PathBuf>,

    /// Additional language-specific configuration (JSON)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub extra_config: Option<serde_json::Value>,
}

fn default_memory_limit() -> u64 {
    512 // 512 MB default
}

fn default_timeout() -> u64 {
    30 // 30 seconds default
}

impl GenericFunctionConfig {
    /// Create a new function configuration
    pub fn new(name: String, language: Language, code: String, handler: String) -> Self {
        Self {
            name,
            language,
            code,
            handler,
            environment: HashMap::new(),
            memory_limit_mb: default_memory_limit(),
            timeout_seconds: default_timeout(),
            working_dir: None,
            extra_config: None,
        }
    }

    /// Set memory limit
    pub fn with_memory_limit(mut self, mb: u64) -> Self {
        self.memory_limit_mb = mb;
        self
    }

    /// Set timeout
    pub fn with_timeout(mut self, seconds: u64) -> Self {
        self.timeout_seconds = seconds;
        self
    }

    /// Add environment variable
    pub fn with_env(mut self, key: String, value: String) -> Self {
        self.environment.insert(key, value);
        self
    }

    /// Set working directory
    pub fn with_working_dir(mut self, dir: PathBuf) -> Self {
        self.working_dir = Some(dir);
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_language_from_str() {
        assert_eq!(Language::from_str("python"), Ok(Language::Python));
        assert_eq!(Language::from_str("Python"), Ok(Language::Python));
        assert_eq!(Language::from_str("python3"), Ok(Language::Python));
        assert_eq!(Language::from_str("nodejs"), Ok(Language::NodeJS));
        assert_eq!(Language::from_str("Node"), Ok(Language::NodeJS));
        assert_eq!(Language::from_str("javascript"), Ok(Language::NodeJS));
        assert_eq!(Language::from_str("java"), Ok(Language::Java));
        assert!(Language::from_str("unknown").is_err());
    }

    #[test]
    fn test_language_display() {
        assert_eq!(Language::Python.to_string(), "python");
        assert_eq!(Language::NodeJS.to_string(), "nodejs");
        assert_eq!(Language::Java.to_string(), "java");
    }

    #[test]
    fn test_generic_function_config() {
        let config = GenericFunctionConfig::new(
            "test-func".to_string(),
            Language::Python,
            "def handler(e, c): pass".to_string(),
            "handler".to_string(),
        )
        .with_memory_limit(256)
        .with_timeout(60)
        .with_env("KEY".to_string(), "value".to_string());

        assert_eq!(config.name, "test-func");
        assert_eq!(config.language, Language::Python);
        assert_eq!(config.memory_limit_mb, 256);
        assert_eq!(config.timeout_seconds, 60);
        assert_eq!(config.environment.get("KEY"), Some(&"value".to_string()));
    }
}
