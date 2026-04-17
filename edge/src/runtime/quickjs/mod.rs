//! QuickJS-based JavaScript Runtime
//!
//! # Architecture
//!
//! This module provides a lightweight JavaScript runtime for edge execution.
//! Since we target Cloudflare Workers WASM, we implement a subset of QuickJS
//! functionality with focus on:
//!
//! - ES2023 syntax support (via AST interpretation)
//! - Console API for logging
//! - JSON operations
//! - setTimeout/setInterval for async operations
//! - Fetch API integration with Workers runtime
//!
//! # Design Decisions
//!
//! | Decision | Rationale |
//! |----------|-----------|
//! | Interpreter-based | Lower memory footprint vs JIT |
//! | No eval() | Security: prevent code injection |
//! | Limited globals | Minimal attack surface |
//! | Async-first | Workers are async by design |

mod ast;
mod builtin;
mod engine;
mod fetch;
mod value;

pub use builtin::{ConsoleApi, JsonApi, TimerApi};
pub use engine::QuickJsEngine;
pub use fetch::{FetchApi, Headers, Request, RequestMethod, Response};
pub use value::{JsArray, JsFunction, JsObject, JsVal};

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// QuickJS runtime configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QuickJsConfig {
    /// Maximum memory in bytes
    pub memory_limit: u32,
    /// Maximum execution time in milliseconds
    pub timeout_ms: u32,
    /// Maximum stack depth
    pub max_stack_depth: u32,
    /// Enable console API
    pub enable_console: bool,
    /// Enable timers (setTimeout, setInterval)
    pub enable_timers: bool,
    /// Enable fetch API
    pub enable_fetch: bool,
}

impl Default for QuickJsConfig {
    fn default() -> Self {
        Self {
            memory_limit: 128 * 1024 * 1024, // 128 MB
            timeout_ms: 30_000,              // 30 seconds
            max_stack_depth: 1000,
            enable_console: true,
            enable_timers: true,
            enable_fetch: true,
        }
    }
}

impl QuickJsConfig {
    /// Create a minimal configuration for quick execution
    pub fn minimal() -> Self {
        Self {
            memory_limit: 16 * 1024 * 1024, // 16 MB
            timeout_ms: 5_000,              // 5 seconds
            max_stack_depth: 100,
            enable_console: true,
            enable_timers: false,
            enable_fetch: false,
        }
    }

    /// Create a full-featured configuration
    pub fn full() -> Self {
        Self {
            memory_limit: 256 * 1024 * 1024, // 256 MB
            timeout_ms: 60_000,              // 60 seconds
            max_stack_depth: 2000,
            enable_console: true,
            enable_timers: true,
            enable_fetch: true,
        }
    }
}

/// Console log captured during execution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConsoleLog {
    /// Log level
    pub level: ConsoleLogLevel,
    /// Log message
    pub message: String,
    /// Additional arguments (JSON serialized)
    pub args: Vec<serde_json::Value>,
    /// Timestamp (milliseconds since execution start)
    pub timestamp_ms: f64,
}

/// Console log levels
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum ConsoleLogLevel {
    Log,
    Debug,
    Info,
    Warn,
    Error,
    Trace,
}

/// Execution statistics
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ExecutionStats {
    /// Total memory allocated (bytes)
    pub memory_used: u32,
    /// Peak memory usage (bytes)
    pub peak_memory: u32,
    /// Number of function calls
    pub function_calls: u32,
    /// Number of allocations
    pub allocations: u32,
    /// Execution time (milliseconds)
    pub execution_time_ms: f64,
    /// Number of async operations
    pub async_ops: u32,
}

/// Result of JavaScript execution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QuickJsResult {
    /// Return value (JSON serialized)
    pub value: serde_json::Value,
    /// Console logs captured
    pub console_logs: Vec<ConsoleLog>,
    /// Execution statistics
    pub stats: ExecutionStats,
    /// Whether execution completed successfully
    pub success: bool,
    /// Error message if failed
    pub error: Option<String>,
}

/// JavaScript runtime error
#[derive(Debug, Clone)]
pub enum QuickJsError {
    /// Syntax error in source code
    SyntaxError {
        message: String,
        line: u32,
        column: u32,
    },
    /// Reference to undefined variable
    ReferenceError { name: String },
    /// Type error (wrong type for operation)
    TypeError { message: String },
    /// Range error (value out of range)
    RangeError { message: String },
    /// Execution timeout
    Timeout { elapsed_ms: u32 },
    /// Out of memory
    OutOfMemory { used: u32, limit: u32 },
    /// Stack overflow
    StackOverflow { depth: u32 },
    /// Internal error
    Internal { message: String },
}

impl std::fmt::Display for QuickJsError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            QuickJsError::SyntaxError {
                message,
                line,
                column,
            } => {
                write!(f, "SyntaxError: {} at line {}:{}", message, line, column)
            }
            QuickJsError::ReferenceError { name } => {
                write!(f, "ReferenceError: {} is not defined", name)
            }
            QuickJsError::TypeError { message } => {
                write!(f, "TypeError: {}", message)
            }
            QuickJsError::RangeError { message } => {
                write!(f, "RangeError: {}", message)
            }
            QuickJsError::Timeout { elapsed_ms } => {
                write!(f, "Execution timeout after {}ms", elapsed_ms)
            }
            QuickJsError::OutOfMemory { used, limit } => {
                write!(f, "Out of memory: used {} of {} bytes", used, limit)
            }
            QuickJsError::StackOverflow { depth } => {
                write!(f, "Stack overflow at depth {}", depth)
            }
            QuickJsError::Internal { message } => {
                write!(f, "Internal error: {}", message)
            }
        }
    }
}

impl std::error::Error for QuickJsError {}

pub type QuickJsResultType<T> = Result<T, QuickJsError>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_defaults() {
        let config = QuickJsConfig::default();
        assert_eq!(config.memory_limit, 128 * 1024 * 1024);
        assert_eq!(config.timeout_ms, 30_000);
        assert!(config.enable_console);
    }

    #[test]
    fn test_config_minimal() {
        let config = QuickJsConfig::minimal();
        assert_eq!(config.memory_limit, 16 * 1024 * 1024);
        assert!(!config.enable_timers);
    }

    #[test]
    fn test_error_display() {
        let err = QuickJsError::SyntaxError {
            message: "unexpected token".to_string(),
            line: 10,
            column: 5,
        };
        assert_eq!(
            err.to_string(),
            "SyntaxError: unexpected token at line 10:5"
        );

        let err = QuickJsError::ReferenceError {
            name: "foo".to_string(),
        };
        assert_eq!(err.to_string(), "ReferenceError: foo is not defined");
    }

    #[test]
    fn test_console_log_serialization() {
        let log = ConsoleLog {
            level: ConsoleLogLevel::Info,
            message: "test".to_string(),
            args: vec![serde_json::json!(42)],
            timestamp_ms: 100.5,
        };
        let json = serde_json::to_string(&log).expect("Should serialize");
        assert!(json.contains("\"level\":\"info\""));
        assert!(json.contains("\"message\":\"test\""));
    }
}
