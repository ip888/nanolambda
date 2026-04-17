//! Sandbox configuration and limits
//!
//! Defines security boundaries for JavaScript execution.

use serde::{Deserialize, Serialize};

/// Sandbox configuration for JavaScript execution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SandboxConfig {
    /// Resource limits
    pub limits: SandboxLimits,
    /// Allowed global APIs
    pub allowed_apis: AllowedApis,
    /// Enable strict mode
    pub strict_mode: bool,
}

impl Default for SandboxConfig {
    fn default() -> Self {
        Self {
            limits: SandboxLimits::default(),
            allowed_apis: AllowedApis::default(),
            strict_mode: true,
        }
    }
}

/// Resource limits for sandbox
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SandboxLimits {
    /// Maximum execution time in milliseconds
    pub max_execution_ms: u32,
    /// Maximum memory in bytes
    pub max_memory_bytes: u32,
    /// Maximum stack size in bytes
    pub max_stack_bytes: u32,
    /// Maximum string length
    pub max_string_length: u32,
    /// Maximum array length
    pub max_array_length: u32,
    /// Maximum recursion depth
    pub max_recursion_depth: u32,
    /// Maximum number of properties per object
    pub max_object_properties: u32,
}

impl Default for SandboxLimits {
    fn default() -> Self {
        Self {
            max_execution_ms: 30_000,           // 30 seconds
            max_memory_bytes: 128 * 1024 * 1024, // 128 MB
            max_stack_bytes: 1024 * 1024,        // 1 MB stack
            max_string_length: 10 * 1024 * 1024, // 10 MB strings
            max_array_length: 1_000_000,         // 1M elements
            max_recursion_depth: 1000,
            max_object_properties: 100_000,
        }
    }
}

impl SandboxLimits {
    /// Create limits for a quick, lightweight execution
    pub fn quick() -> Self {
        Self {
            max_execution_ms: 5_000,            // 5 seconds
            max_memory_bytes: 32 * 1024 * 1024, // 32 MB
            max_stack_bytes: 256 * 1024,        // 256 KB stack
            max_string_length: 1024 * 1024,     // 1 MB strings
            max_array_length: 100_000,
            max_recursion_depth: 100,
            max_object_properties: 10_000,
        }
    }

    /// Create limits for heavy computation
    pub fn heavy() -> Self {
        Self {
            max_execution_ms: 300_000,           // 5 minutes
            max_memory_bytes: 512 * 1024 * 1024, // 512 MB
            max_stack_bytes: 4 * 1024 * 1024,    // 4 MB stack
            max_string_length: 100 * 1024 * 1024, // 100 MB strings
            max_array_length: 10_000_000,
            max_recursion_depth: 10_000,
            max_object_properties: 1_000_000,
        }
    }
}

/// Allowed JavaScript APIs in the sandbox
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AllowedApis {
    /// Allow console API (log, warn, error, etc.)
    pub console: bool,
    /// Allow setTimeout/setInterval
    pub timers: bool,
    /// Allow fetch API
    pub fetch: bool,
    /// Allow TextEncoder/TextDecoder
    pub text_encoding: bool,
    /// Allow crypto.subtle
    pub crypto: bool,
    /// Allow URL/URLSearchParams
    pub url: bool,
    /// Allow atob/btoa
    pub base64: bool,
    /// Allow WebAssembly
    pub wasm: bool,
}

impl Default for AllowedApis {
    fn default() -> Self {
        Self {
            console: true,
            timers: true,
            fetch: true,
            text_encoding: true,
            crypto: true,
            url: true,
            base64: true,
            wasm: false, // Disabled by default for security
        }
    }
}

impl AllowedApis {
    /// Minimal API set (just console)
    pub fn minimal() -> Self {
        Self {
            console: true,
            timers: false,
            fetch: false,
            text_encoding: true,
            crypto: false,
            url: false,
            base64: true,
            wasm: false,
        }
    }

    /// Full API set (everything enabled)
    pub fn full() -> Self {
        Self {
            console: true,
            timers: true,
            fetch: true,
            text_encoding: true,
            crypto: true,
            url: true,
            base64: true,
            wasm: true,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_limits() {
        let limits = SandboxLimits::default();
        assert_eq!(limits.max_execution_ms, 30_000);
        assert_eq!(limits.max_memory_bytes, 128 * 1024 * 1024);
    }

    #[test]
    fn test_quick_limits() {
        let limits = SandboxLimits::quick();
        assert_eq!(limits.max_execution_ms, 5_000);
        assert!(limits.max_memory_bytes < SandboxLimits::default().max_memory_bytes);
    }

    #[test]
    fn test_default_apis() {
        let apis = AllowedApis::default();
        assert!(apis.console);
        assert!(apis.fetch);
        assert!(!apis.wasm); // Disabled by default
    }

    #[test]
    fn test_serialization() {
        let config = SandboxConfig::default();
        let json = serde_json::to_string(&config).expect("Should serialize");
        let parsed: SandboxConfig = serde_json::from_str(&json).expect("Should deserialize");
        assert_eq!(parsed.limits.max_execution_ms, config.limits.max_execution_ms);
    }
}
