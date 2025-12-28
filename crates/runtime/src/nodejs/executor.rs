//! Node.js executor implementing the Runtime trait
//!
//! Provides the main NodeJSExecutor that implements the Runtime trait for executing
//! JavaScript/TypeScript functions with warm start support via process pooling.

use super::{NodeVersion, detect_nodejs};
use crate::nodejs::process::NodeProcess;
use crate::runtime_trait::Runtime;
use crate::types::{GenericFunctionConfig, Language, RuntimeCapabilities, RuntimeInfo};
use crate::{ExecutionMetrics, ExecutionResult, ExecutorError};
use async_trait::async_trait;
use serde_json::Value;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::time::Instant;

/// Result type for executor operations
type Result<T> = std::result::Result<T, ExecutorError>;

/// Process pool for warm starts
struct ProcessPool {
    processes: HashMap<String, NodeProcess>,
    max_size: usize,
    max_age_seconds: u64,
}

impl ProcessPool {
    fn new(max_size: usize, max_age_seconds: u64) -> Self {
        ProcessPool {
            processes: HashMap::new(),
            max_size,
            max_age_seconds,
        }
    }

    /// Get or create a process for the given code
    fn get_or_create(
        &mut self,
        code_hash: &str,
        node_path: &str,
        function_code: &str,
    ) -> Result<&mut NodeProcess> {
        // Clean up old or unhealthy processes
        self.cleanup(node_path, function_code);

        // Check if we have an existing process for this code
        let exists = self.processes.contains_key(code_hash);

        if exists {
            // Check if it's healthy
            let is_healthy = self
                .processes
                .get_mut(code_hash)
                .map(|p| p.is_healthy())
                .unwrap_or(false);
            if is_healthy {
                return self
                    .processes
                    .get_mut(code_hash)
                    .ok_or_else(|| ExecutorError::RuntimeError("Process disappeared".to_string()));
            } else {
                // Remove unhealthy process
                self.processes.remove(code_hash);
            }
        }

        // Create new process
        let process = NodeProcess::new(node_path, function_code, code_hash.to_string())
            .map_err(|e| ExecutorError::RuntimeError(format!("Failed to create process: {}", e)))?;

        self.processes.insert(code_hash.to_string(), process);
        self.processes.get_mut(code_hash).ok_or_else(|| {
            ExecutorError::RuntimeError("Failed to retrieve inserted process".to_string())
        })
    }

    /// Clean up old or unhealthy processes
    fn cleanup(&mut self, _node_path: &str, _function_code: &str) {
        let max_age = std::time::Duration::from_secs(self.max_age_seconds);

        // Remove unhealthy or old processes
        self.processes
            .retain(|_, process| process.is_healthy() && process.age() < max_age);

        // Enforce max size
        while self.processes.len() > self.max_size {
            // Remove the oldest process
            if let Some(oldest) = self
                .processes
                .iter()
                .max_by_key(|(_, p)| p.age())
                .map(|(k, _)| k.clone())
            {
                self.processes.remove(&oldest);
            } else {
                break;
            }
        }
    }
}

/// Node.js runtime executor
pub struct NodeJSExecutor {
    node_path: PathBuf,
    node_version: NodeVersion,
    pool: Arc<Mutex<ProcessPool>>,
    enable_warm_starts: bool,
}

impl NodeJSExecutor {
    /// Create a new Node.js executor
    pub fn new() -> Result<Self> {
        let (node_path, node_version) = detect_nodejs()
            .map_err(|e| ExecutorError::RuntimeError(format!("Node.js detection failed: {}", e)))?;

        Ok(NodeJSExecutor {
            node_path,
            node_version,
            pool: Arc::new(Mutex::new(ProcessPool::new(10, 300))), // 10 processes, 5 min max age
            enable_warm_starts: true,
        })
    }

    /// Create with custom configuration
    pub fn with_config(max_pool_size: usize, max_age_seconds: u64) -> Result<Self> {
        let (node_path, node_version) = detect_nodejs()
            .map_err(|e| ExecutorError::RuntimeError(format!("Node.js detection failed: {}", e)))?;

        Ok(NodeJSExecutor {
            node_path,
            node_version,
            pool: Arc::new(Mutex::new(ProcessPool::new(max_pool_size, max_age_seconds))),
            enable_warm_starts: true,
        })
    }

    /// Execute with cold start (no pooling)
    fn execute_cold_start(&self, function_code: &str, event: &Value) -> Result<ExecutionResult> {
        let start = Instant::now();

        let node_path = self.node_path.to_string_lossy();
        let code_hash = blake3::hash(function_code.as_bytes()).to_hex();

        let mut process = NodeProcess::new(&node_path, function_code, code_hash.to_string())
            .map_err(|e| ExecutorError::RuntimeError(format!("Failed to spawn process: {}", e)))?;

        let invocation_result = process
            .invoke(event)
            .map_err(|e| ExecutorError::RuntimeError(format!("Invocation failed: {}", e)))?;

        let total_time = start.elapsed().as_millis() as u64;

        Ok(ExecutionResult {
            success: invocation_result.success,
            result: invocation_result.result.map(|v| v.to_string()),
            error: invocation_result.error,
            metrics: ExecutionMetrics {
                cold_start_ms: total_time,
                execution_ms: invocation_result.execution_ms,
                total_ms: total_time,
                memory_peak_mb: invocation_result.memory_used_mb.unwrap_or(0.0),
                cpu_percent: invocation_result.cpu_percent.unwrap_or(0.0),
                exit_code: 0,
                stdout: String::new(),
                stderr: String::new(),
                is_cold_start: true,
                python_version: format!("Node.js {}", self.node_version.full),
            },
        })
    }

    /// Execute with warm start (use pooling)
    fn execute_warm_start(&self, function_code: &str, event: &Value) -> Result<ExecutionResult> {
        let start = Instant::now();

        let code_hash = blake3::hash(function_code.as_bytes()).to_hex();
        let node_path = self.node_path.to_string_lossy();

        let mut pool = self
            .pool
            .lock()
            .map_err(|e| ExecutorError::RuntimeError(format!("Mutex poisoned: {}", e)))?;
        let process = pool.get_or_create(&code_hash, &node_path, function_code)?;

        // Check if this is a cold start (new process)
        let is_cold_start = process.stats().invocations == 0;

        let invocation_result = process
            .invoke(event)
            .map_err(|e| ExecutorError::RuntimeError(format!("Invocation failed: {}", e)))?;

        let total_time = start.elapsed().as_millis() as u64;

        Ok(ExecutionResult {
            success: invocation_result.success,
            result: invocation_result.result.map(|v| v.to_string()),
            error: invocation_result.error,
            metrics: ExecutionMetrics {
                cold_start_ms: if is_cold_start { total_time } else { 0 },
                execution_ms: invocation_result.execution_ms,
                total_ms: total_time,
                memory_peak_mb: invocation_result.memory_used_mb.unwrap_or(0.0),
                cpu_percent: invocation_result.cpu_percent.unwrap_or(0.0),
                exit_code: 0,
                stdout: String::new(),
                stderr: String::new(),
                is_cold_start,
                python_version: format!("Node.js {}", self.node_version.full),
            },
        })
    }
}

#[async_trait]
impl Runtime for NodeJSExecutor {
    async fn execute(
        &self,
        config: &GenericFunctionConfig,
        event: Value,
    ) -> Result<ExecutionResult> {
        // Validate language
        if config.language != Language::NodeJS {
            return Err(ExecutorError::UnsupportedLanguage(format!(
                "Expected NodeJS, got {:?}",
                config.language
            )));
        }

        // Execute based on warm start setting
        if self.enable_warm_starts {
            self.execute_warm_start(&config.code, &event)
        } else {
            self.execute_cold_start(&config.code, &event)
        }
    }

    fn runtime_info(&self) -> RuntimeInfo {
        RuntimeInfo {
            language: Language::NodeJS,
            version: self.node_version.full.clone(),
            interpreter_path: self.node_path.clone(),
            capabilities: RuntimeCapabilities {
                warm_starts: true,
                async_execution: true,
                streaming: false,
                max_memory_mb: None,
                max_timeout_seconds: None,
            },
        }
    }

    fn health_check(&self) -> Result<()> {
        // Check if node binary exists and is executable
        if !self.node_path.exists() {
            return Err(ExecutorError::RuntimeError(
                "Node.js binary not found".to_string(),
            ));
        }

        // Try to execute node --version
        std::process::Command::new(&self.node_path)
            .arg("--version")
            .output()
            .map_err(|e| ExecutorError::RuntimeError(format!("Health check failed: {}", e)))?;

        Ok(())
    }

    fn set_warm_starts(&mut self, enabled: bool) {
        self.enable_warm_starts = enabled;
    }

    fn warm_starts_enabled(&self) -> bool {
        self.enable_warm_starts
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn get_executor() -> Option<NodeJSExecutor> {
        NodeJSExecutor::new().ok()
    }

    #[tokio::test]
    async fn test_executor_creation() {
        if let Some(executor) = get_executor() {
            let info = executor.runtime_info();
            assert_eq!(info.language, Language::NodeJS);
            assert!(info.version.starts_with('v'));
            assert!(info.capabilities.warm_starts);
            assert!(info.capabilities.async_execution);
        } else {
            println!("Node.js not found - skipping test");
        }
    }

    #[tokio::test]
    async fn test_health_check() {
        if let Some(executor) = get_executor() {
            assert!(executor.health_check().is_ok());
        } else {
            println!("Node.js not found - skipping test");
        }
    }

    #[tokio::test]
    async fn test_simple_execution() {
        let executor = match get_executor() {
            Some(e) => e,
            None => {
                println!("Node.js not found - skipping test");
                return;
            }
        };

        let config = GenericFunctionConfig::new(
            "test-function".to_string(),
            Language::NodeJS,
            r#"
                exports.handler = (event) => {
                    return { message: `Hello, ${event.name}!` };
                };
            "#
            .to_string(),
            "handler".to_string(),
        );

        let event = json!({ "name": "World" });
        let result = executor.execute(&config, event).await.unwrap();

        assert!(result.success);
        let result_json: Value = serde_json::from_str(result.result.as_ref().unwrap()).unwrap();
        assert_eq!(result_json, json!({ "message": "Hello, World!" }));
    }

    #[tokio::test]
    async fn test_async_execution() {
        let executor = match get_executor() {
            Some(e) => e,
            None => {
                println!("Node.js not found - skipping test");
                return;
            }
        };

        let config = GenericFunctionConfig::new(
            "async-function".to_string(),
            Language::NodeJS,
            r#"
                exports.handler = async (event) => {
                    await new Promise(resolve => setTimeout(resolve, 10));
                    return { result: event.value * 2 };
                };
            "#
            .to_string(),
            "handler".to_string(),
        );

        let event = json!({ "value": 21 });
        let result = executor.execute(&config, event).await.unwrap();

        assert!(result.success);
        let result_json: Value = serde_json::from_str(result.result.as_ref().unwrap()).unwrap();
        assert_eq!(result_json, json!({ "result": 42 }));
    }

    #[tokio::test]
    async fn test_error_handling() {
        let executor = match get_executor() {
            Some(e) => e,
            None => {
                println!("Node.js not found - skipping test");
                return;
            }
        };

        let config = GenericFunctionConfig::new(
            "error-function".to_string(),
            Language::NodeJS,
            r#"
                exports.handler = (event) => {
                    throw new Error('Test error');
                };
            "#
            .to_string(),
            "handler".to_string(),
        );

        let event = json!({});
        let result = executor.execute(&config, event).await.unwrap();

        assert!(!result.success);
        assert!(result.error.is_some());
        assert!(result.error.unwrap().contains("Test error"));
    }

    #[tokio::test]
    async fn test_warm_starts() {
        let executor = match get_executor() {
            Some(e) => e,
            None => {
                println!("Node.js not found - skipping test");
                return;
            }
        };

        let config = GenericFunctionConfig::new(
            "warm-function".to_string(),
            Language::NodeJS,
            r#"
                let counter = 0;
                exports.handler = (event) => {
                    counter++;
                    return { counter };
                };
            "#
            .to_string(),
            "handler".to_string(),
        );

        // First execution (cold start)
        let result1 = executor.execute(&config, json!({})).await.unwrap();
        assert!(result1.success);
        let result1_json: Value = serde_json::from_str(result1.result.as_ref().unwrap()).unwrap();
        assert_eq!(result1_json, json!({ "counter": 1 }));
        assert!(result1.metrics.is_cold_start);

        // Second execution (warm start - should reuse process)
        let result2 = executor.execute(&config, json!({})).await.unwrap();
        assert!(result2.success);
        let result2_json: Value = serde_json::from_str(result2.result.as_ref().unwrap()).unwrap();
        assert_eq!(result2_json, json!({ "counter": 2 }));
        assert!(!result2.metrics.is_cold_start); // Should be warm

        // Execution time should be faster
        assert!(result2.metrics.total_ms <= result1.metrics.total_ms);
    }

    #[tokio::test]
    async fn test_cold_start_mode() {
        let mut executor = match get_executor() {
            Some(e) => e,
            None => {
                println!("Node.js not found - skipping test");
                return;
            }
        };

        // Disable warm starts
        executor.set_warm_starts(false);
        assert!(!executor.warm_starts_enabled());

        let config = GenericFunctionConfig::new(
            "cold-function".to_string(),
            Language::NodeJS,
            r#"
                let counter = 0;
                exports.handler = (event) => {
                    counter++;
                    return { counter };
                };
            "#
            .to_string(),
            "handler".to_string(),
        );

        // First execution
        let result1 = executor.execute(&config, json!({})).await.unwrap();
        assert!(result1.success);
        let result1_json: Value = serde_json::from_str(result1.result.as_ref().unwrap()).unwrap();
        assert_eq!(result1_json, json!({ "counter": 1 }));
        assert!(result1.metrics.is_cold_start);

        // Second execution (should also be cold start - new process each time)
        let result2 = executor.execute(&config, json!({})).await.unwrap();
        assert!(result2.success);
        let result2_json: Value = serde_json::from_str(result2.result.as_ref().unwrap()).unwrap();
        assert_eq!(result2_json, json!({ "counter": 1 })); // Counter resets
        assert!(result2.metrics.is_cold_start);
    }
}
