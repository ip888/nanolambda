//! QuickJS JavaScript Runtime for Edge Functions
//!
//! # Business Decision: Why QuickJS over V8/SpiderMonkey?
//!
//! | Aspect | QuickJS (Our Choice) | V8 | SpiderMonkey |
//! |--------|---------------------|-----|--------------|
//! | **WASM Size** | ~200KB | 5MB+ | 3MB+ |
//! | **Cold Start** | <5ms | 50ms+ | 30ms+ |
//! | **Memory** | ~256KB baseline | 2MB+ | 1MB+ |
//! | **ES2023** | ✅ Full support | ✅ | ✅ |
//! | **Async/Await** | ✅ | ✅ | ✅ |
//!
//! # Implementation Phases
//!
//! 1. **Phase 1 (Current)**: Basic JS execution, console API, JSON
//! 2. **Phase 2**: Fetch API, setTimeout/setInterval  
//! 3. **Phase 3**: KV/R2 bindings via host functions
//! 4. **Phase 4**: Full Workers API compatibility

mod bindings;
mod context;
pub mod quickjs;
mod sandbox;

pub use bindings::JsBindings;
pub use context::{JsContext, JsError, JsResult, JsValue};
pub use quickjs::{QuickJsConfig, QuickJsEngine, QuickJsError, QuickJsResult};
pub use sandbox::{SandboxConfig, SandboxLimits};

use crate::error::EdgeError;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// JavaScript function execution request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsExecutionRequest {
    /// JavaScript source code
    pub code: String,
    /// Handler function name to invoke
    pub handler: String,
    /// Input event/payload
    pub event: serde_json::Value,
    /// Environment variables
    #[serde(default)]
    pub env: HashMap<String, String>,
    /// Execution timeout in milliseconds
    #[serde(default = "default_timeout")]
    pub timeout_ms: u32,
    /// Memory limit in bytes
    #[serde(default = "default_memory")]
    pub memory_limit: u32,
}

fn default_timeout() -> u32 {
    30_000 // 30 seconds
}

fn default_memory() -> u32 {
    128 * 1024 * 1024 // 128 MB
}

/// JavaScript execution result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsExecutionResult {
    /// Function return value (JSON serialized)
    pub result: serde_json::Value,
    /// Console output logs
    pub logs: Vec<LogEntry>,
    /// Execution duration in milliseconds
    pub duration_ms: f64,
    /// Memory used in bytes
    pub memory_used: u32,
}

/// Console log entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogEntry {
    pub level: LogLevel,
    pub message: String,
    pub timestamp: u64,
}

/// Log levels
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum LogLevel {
    Debug,
    Info,
    Warn,
    Error,
}

/// QuickJS-based JavaScript runtime
///
/// # Safety
///
/// This runtime executes untrusted JavaScript code. Security is ensured by:
/// 1. Memory limits enforced by QuickJS
/// 2. Execution timeout via interrupt callback
/// 3. No access to filesystem, network (unless explicitly bound)
/// 4. Sandboxed global object
pub struct JsRuntime {
    /// Sandbox configuration
    sandbox_config: SandboxConfig,
    /// Pre-compiled modules cache
    module_cache: HashMap<String, Vec<u8>>,
}

impl JsRuntime {
    /// Create a new JavaScript runtime
    pub fn new(sandbox_config: SandboxConfig) -> Self {
        Self {
            sandbox_config,
            module_cache: HashMap::new(),
        }
    }

    /// Execute JavaScript code
    pub async fn execute(
        &self,
        request: JsExecutionRequest,
    ) -> Result<JsExecutionResult, EdgeError> {
        let start = web_time::Instant::now();
        let logs = Vec::new();

        // Create execution context
        let mut context = JsContext::new(&self.sandbox_config)?;

        // Set up environment variables
        context.setup_env(&request.env)?;

        // Compile and evaluate the module
        context.eval_module(&request.code, "function.js")?;

        // Get the handler function
        let handler = context.get_handler(&request.handler)?;

        // Create the event object
        let event = context.create_value(&request.event)?;

        // Call the handler with timeout
        let result = context
            .call_with_timeout(&handler, &[event], request.timeout_ms)
            .await?;

        // Serialize the result
        let result_json = context.to_json(&result)?;

        let duration = start.elapsed();

        Ok(JsExecutionResult {
            result: result_json,
            logs,
            duration_ms: duration.as_secs_f64() * 1000.0,
            memory_used: context.memory_used(),
        })
    }

    /// Validate JavaScript code without executing
    pub fn validate(&self, code: &str) -> Result<(), EdgeError> {
        let context = JsContext::new(&self.sandbox_config)?;
        context.compile(code, "validation.js")?;
        Ok(())
    }

    /// Precompile a module for caching
    pub fn precompile(&mut self, name: &str, code: &str) -> Result<(), EdgeError> {
        let context = JsContext::new(&self.sandbox_config)?;
        let bytecode = context.compile_to_bytecode(code, name)?;
        self.module_cache.insert(name.to_string(), bytecode);
        Ok(())
    }
}

impl Default for JsRuntime {
    fn default() -> Self {
        Self::new(SandboxConfig::default())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_execution_request_defaults() {
        let json = r#"{"code": "console.log('hi')", "handler": "fetch", "event": {}}"#;
        let request: JsExecutionRequest = serde_json::from_str(json).expect("Should parse");
        assert_eq!(request.timeout_ms, 30_000);
        assert_eq!(request.memory_limit, 128 * 1024 * 1024);
    }

    #[test]
    fn test_log_level_serialization() {
        let entry = LogEntry {
            level: LogLevel::Info,
            message: "test".to_string(),
            timestamp: 0,
        };
        let json = serde_json::to_string(&entry).expect("Should serialize");
        assert!(json.contains("\"level\":\"info\""));
    }
}
