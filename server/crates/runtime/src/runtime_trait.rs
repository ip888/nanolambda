//! Runtime trait for language-agnostic function execution

use serde_json::Value;

use crate::executor::{ExecutionResult, ExecutorError};
use crate::types::{GenericFunctionConfig, RuntimeInfo};

/// Generic runtime interface for executing functions in any language
///
/// This trait provides a consistent API for executing serverless functions
/// regardless of the underlying programming language (Python, Node.js, Java, etc.)
pub trait Runtime: Send + Sync {
    /// Execute a function with the given configuration and event
    ///
    /// # Arguments
    /// * `config` - Function configuration including code, handler, environment
    /// * `event` - Input event as JSON value
    ///
    /// # Returns
    /// Execution result with output, metrics, and error information
    fn execute(
        &self,
        config: &GenericFunctionConfig,
        event: Value,
    ) -> impl std::future::Future<Output = Result<ExecutionResult, ExecutorError>> + Send;

    /// Get runtime information (language, version, capabilities)
    fn runtime_info(&self) -> RuntimeInfo;

    /// Check if the runtime is healthy and available
    ///
    /// Verifies that the interpreter/runtime is installed and accessible
    fn health_check(&self) -> Result<(), ExecutorError>;

    /// Enable or disable warm starts (process reuse)
    fn set_warm_starts(&mut self, enabled: bool);

    /// Check if warm starts are enabled
    fn warm_starts_enabled(&self) -> bool;
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::executor::ExecutionMetrics;
    use crate::types::{Language, RuntimeCapabilities};
    use std::path::PathBuf;

    // Mock runtime for testing
    struct MockRuntime {
        warm_starts: bool,
    }

    impl Runtime for MockRuntime {
        async fn execute(
            &self,
            _config: &GenericFunctionConfig,
            _event: Value,
        ) -> Result<ExecutionResult, ExecutorError> {
            Ok(ExecutionResult {
                success: true,
                result: Some(r#"{"message": "test"}"#.to_string()),
                error: None,
                metrics: ExecutionMetrics {
                    cold_start_ms: 0,
                    execution_ms: 10,
                    total_ms: 10,
                    memory_peak_mb: 50.0,
                    cpu_percent: 5.0,
                    exit_code: 0,
                    stdout: String::new(),
                    stderr: String::new(),
                    is_cold_start: false,
                    python_version: "mock".to_string(),
                },
            })
        }

        fn runtime_info(&self) -> RuntimeInfo {
            RuntimeInfo {
                language: Language::Python,
                version: "mock".to_string(),
                interpreter_path: PathBuf::from("/usr/bin/mock"),
                capabilities: RuntimeCapabilities::default(),
            }
        }

        fn health_check(&self) -> Result<(), ExecutorError> {
            Ok(())
        }

        fn set_warm_starts(&mut self, enabled: bool) {
            self.warm_starts = enabled;
        }

        fn warm_starts_enabled(&self) -> bool {
            self.warm_starts
        }
    }

    #[tokio::test]
    async fn test_sync_runtime_trait() {
        let runtime = MockRuntime { warm_starts: false };
        let config = GenericFunctionConfig::new(
            "test".to_string(),
            Language::Python,
            "code".to_string(),
            "handler".to_string(),
        );

        let result = runtime.execute(&config, Value::Null).await;
        assert!(result.is_ok());

        let result = result.unwrap();
        assert!(result.success);
        assert_eq!(result.result, Some(r#"{"message": "test"}"#.to_string()));
    }

    #[test]
    fn test_runtime_info() {
        let runtime = MockRuntime { warm_starts: false };
        let info = runtime.runtime_info();

        assert_eq!(info.language, Language::Python);
        assert_eq!(info.version, "mock");
    }
}
