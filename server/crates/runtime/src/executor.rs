//! Real Python function executor with metrics collection
//!
//! This module provides actual Python execution with:
//! - Process isolation and resource limits
//! - Comprehensive metrics (cold start, execution time, memory, CPU)
//! - Support for Python 3.11 and 3.12
//! - Proper error handling and timeout management

use crate::pool::ProcessPool;
use base64::{Engine as _, engine::general_purpose};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::time::{Duration, Instant, SystemTime};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum ExecutorError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Function execution failed: {0}")]
    ExecutionFailed(String),

    #[error("Function timeout after {0:?}")]
    Timeout(Duration),

    #[error("Invalid function code: {0}")]
    InvalidCode(String),

    #[error("Python not found: {0}")]
    PythonNotFound(String),

    #[error("Memory limit exceeded: {0} MB")]
    MemoryLimitExceeded(u64),

    #[error("Runtime error: {0}")]
    RuntimeError(String),

    #[error("Unsupported language: {0}")]
    UnsupportedLanguage(String),
}

pub type Result<T> = std::result::Result<T, ExecutorError>;

/// Execution metrics collected during function invocation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionMetrics {
    /// Cold start time in milliseconds (environment setup)
    pub cold_start_ms: u64,

    /// Function execution time in milliseconds
    pub execution_ms: u64,

    /// Total time (cold start + execution) in milliseconds
    pub total_ms: u64,

    /// Peak memory usage in MB
    pub memory_peak_mb: f64,

    /// Average CPU usage percentage (0-100)
    pub cpu_percent: f64,

    /// Exit code of the process
    pub exit_code: i32,

    /// Standard output from the function
    pub stdout: String,

    /// Standard error from the function
    pub stderr: String,

    /// Whether this was a cold start
    pub is_cold_start: bool,

    /// Python version used
    pub python_version: String,
}

/// Python function configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FunctionConfig {
    /// Function ID
    pub id: i64,

    /// Function version
    pub version: i64,

    /// Function name/identifier
    pub name: String,

    /// Python code as a string
    pub code: String,

    /// Handler function name (e.g., "handler" or "lambda_handler")
    pub handler: String,

    /// Environment variables
    pub environment: HashMap<String, String>,

    /// Memory limit in MB
    pub memory_limit_mb: u64,

    /// Execution timeout in seconds
    pub timeout_seconds: u64,

    /// Working directory (optional, will be created if not provided)
    pub working_dir: Option<PathBuf>,
}

/// Result of a function execution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionResult {
    /// Whether the execution was successful
    pub success: bool,

    /// Function return value (JSON serialized)
    pub result: Option<String>,

    /// Error message if execution failed
    pub error: Option<String>,

    /// Execution metrics
    pub metrics: ExecutionMetrics,
}

/// Real Python function executor
pub struct PythonExecutor {
    /// Python interpreter path
    python_path: PathBuf,

    /// Python version string
    python_version: String,

    /// Base working directory for functions
    base_dir: PathBuf,

    /// Whether to keep function directories after execution (for debugging)
    keep_dirs: bool,

    /// Optional process pool for warm starts
    pool: Option<ProcessPool>,

    /// Whether to use warm starts (process reuse)
    enable_warm_starts: bool,
}

impl PythonExecutor {
    /// Create a new Python executor
    pub fn new() -> Result<Self> {
        Self::with_base_dir(PathBuf::from("/tmp/nanolambda"))
    }

    /// Create a new Python executor with custom base directory
    pub fn with_base_dir(base_dir: PathBuf) -> Result<Self> {
        // Detect Python installation
        let (python_path, python_version) = Self::detect_python()?;

        // Create base directory
        fs::create_dir_all(&base_dir)?;

        // Create process pool for warm starts
        let pool = ProcessPool::new(python_path.to_string_lossy().to_string());

        Ok(Self {
            python_path,
            python_version,
            base_dir,
            keep_dirs: false,
            pool: Some(pool),
            enable_warm_starts: true, // Enabled - ProcessPool now supports dynamic handler names
        })
    }

    /// Disable warm starts (use cold starts only)
    pub fn disable_warm_starts(&mut self) {
        self.enable_warm_starts = false;
    }

    /// Enable warm starts (use process pool)
    pub fn enable_warm_starts(&mut self) {
        self.enable_warm_starts = true;
    }

    /// Detect available Python installation
    fn detect_python() -> Result<(PathBuf, String)> {
        // Try latest Python versions first: 3.13, 3.12, 3.11, then python3
        for python_cmd in &["python3.13", "python3.12", "python3.11", "python3"] {
            if let Ok(output) = Command::new(python_cmd).arg("--version").output()
                && output.status.success()
            {
                let version = String::from_utf8_lossy(&output.stdout).trim().to_string();

                // Get full path
                if let Ok(which_output) = Command::new("which").arg(python_cmd).output() {
                    let path = String::from_utf8_lossy(&which_output.stdout)
                        .trim()
                        .to_string();

                    return Ok((PathBuf::from(path), version));
                }
            }
        }

        Err(ExecutorError::PythonNotFound(
            "Could not find Python 3.11 or 3.12. Please install Python.".to_string(),
        ))
    }

    /// Execute a Python function with full metrics collection
    pub fn execute(
        &self,
        config: FunctionConfig,
        event: serde_json::Value,
    ) -> Result<ExecutionResult> {
        let start_time = Instant::now();

        // Try warm start if enabled and pool is available
        if self.enable_warm_starts
            && self.pool.is_some()
            && let Some(ref pool) = self.pool
        {
            match pool.execute_warm(
                config.id,
                config.version,
                &config.code,
                &config.handler,
                &event,
            ) {
                Ok(warm_result) => {
                    let total_ms = start_time.elapsed().as_millis() as u64;
                    let cold_start_ms = if warm_result.is_cold_start {
                        total_ms - warm_result.execution_ms
                    } else {
                        0
                    };

                    return Ok(ExecutionResult {
                        success: warm_result.success,
                        result: if warm_result.success {
                            Some(warm_result.result)
                        } else {
                            None
                        },
                        error: warm_result.error,
                        metrics: ExecutionMetrics {
                            cold_start_ms,
                            execution_ms: warm_result.execution_ms,
                            total_ms,
                            memory_peak_mb: warm_result.peak_memory_mb as f64,
                            cpu_percent: warm_result.cpu_percent,
                            exit_code: if warm_result.success { 0 } else { 1 },
                            stdout: String::new(),
                            stderr: String::new(),
                            is_cold_start: warm_result.is_cold_start,
                            python_version: self.python_version.clone(),
                        },
                    });
                }
                Err(e) => {
                    // Fall back to cold start if warm start fails
                    eprintln!("Warm start failed, falling back to cold start: {}", e);
                }
            }
        }

        // Fall back to cold start (original implementation)
        let is_cold_start = true;

        // Create working directory for this function
        let func_dir = self.create_function_dir(&config)?;

        // Measure cold start time (environment setup)
        let cold_start_begin = Instant::now();

        // Write function code to file
        let func_file = func_dir.join("function.py");
        fs::write(&func_file, &config.code)?;

        // Create execution wrapper
        let wrapper_script = self.create_wrapper_script(&config, &event)?;
        let wrapper_file = func_dir.join("wrapper.py");
        fs::write(&wrapper_file, wrapper_script)?;

        let cold_start_ms = cold_start_begin.elapsed().as_millis() as u64;

        // Execute the function
        let exec_start = Instant::now();

        let mut cmd = Command::new(&self.python_path);
        cmd.arg(&wrapper_file)
            .current_dir(&func_dir)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());

        // Set environment variables
        for (key, value) in &config.environment {
            cmd.env(key, value);
        }

        // Set memory limit using ulimit (if available)
        // This works on Unix-like systems without requiring elevated privileges
        if config.memory_limit_mb > 0 {
            // Convert MB to bytes
            let memory_bytes = config.memory_limit_mb * 1024 * 1024;
            // Use preexec to set resource limits (Unix only)
            // SAFETY: pre_exec runs in the child process after fork() before exec().
            // setrlimit is async-signal-safe and the closure only calls it plus eprintln.
            #[cfg(unix)]
            #[allow(unsafe_code)]
            {
                use std::os::unix::process::CommandExt;
                unsafe {
                    cmd.pre_exec(move || {
                        use libc::{RLIMIT_AS, rlimit, setrlimit};
                        let limit = rlimit {
                            rlim_cur: memory_bytes,
                            rlim_max: memory_bytes,
                        };
                        if setrlimit(RLIMIT_AS, &limit) != 0 {
                            // Non-fatal: log but continue
                            eprintln!("Warning: Failed to set memory limit");
                        }
                        Ok(())
                    });
                }
            }
        }

        // Spawn the process
        let child = cmd.spawn()?;
        let pid = child.id();

        // Wait for completion with timeout
        let timeout = Duration::from_secs(config.timeout_seconds);
        let output = self.wait_with_timeout(child, timeout)?;

        let execution_ms = exec_start.elapsed().as_millis() as u64;
        let total_ms = start_time.elapsed().as_millis() as u64;

        // Parse output
        let stdout = String::from_utf8_lossy(&output.stdout).to_string();
        let stderr = String::from_utf8_lossy(&output.stderr).to_string();

        // Collect real memory and CPU metrics from /proc if available
        let (memory_peak_mb, cpu_percent) =
            if let Ok(metrics) = crate::metrics::ProcessMetrics::from_pid(pid) {
                let memory_mb = (metrics.rss_peak_bytes as f64) / (1024.0 * 1024.0);
                // Calculate CPU percentage: (cpu_time / elapsed_time) * 100
                let total_cpu_jiffies = metrics.cpu_utime + metrics.cpu_stime;
                let clock_ticks_per_sec = 100.0; // sysconf(_SC_CLK_TCK) typically 100
                let cpu_seconds = (total_cpu_jiffies as f64) / clock_ticks_per_sec;
                let elapsed_seconds = execution_ms as f64 / 1000.0;
                let cpu_pct = if elapsed_seconds > 0.0 {
                    (cpu_seconds / elapsed_seconds) * 100.0
                } else {
                    0.0
                };
                (memory_mb, cpu_pct)
            } else {
                // Fallback to estimation
                (self.estimate_memory_usage(&stdout, &stderr), 0.0)
            };

        // Parse result
        let (success, result, error) =
            self.parse_execution_output(&stdout, &stderr, output.status.code());

        // Cleanup
        if !self.keep_dirs {
            let _ = fs::remove_dir_all(&func_dir);
        }

        Ok(ExecutionResult {
            success,
            result,
            error,
            metrics: ExecutionMetrics {
                cold_start_ms,
                execution_ms,
                total_ms,
                memory_peak_mb,
                cpu_percent,
                exit_code: output.status.code().unwrap_or(-1),
                stdout: stdout.clone(),
                stderr: stderr.clone(),
                is_cold_start, // This is a cold start when using subprocess
                python_version: self.python_version.clone(),
            },
        })
    }

    /// Create a unique directory for this function execution
    fn create_function_dir(&self, config: &FunctionConfig) -> Result<PathBuf> {
        let timestamp = SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_micros();

        let dir_name = format!("{}_{}", config.name, timestamp);
        let func_dir = self.base_dir.join(dir_name);

        fs::create_dir_all(&func_dir)?;

        Ok(func_dir)
    }

    /// Create wrapper script that handles event injection and result capture
    fn create_wrapper_script(
        &self,
        config: &FunctionConfig,
        event: &serde_json::Value,
    ) -> Result<String> {
        let event_json = serde_json::to_string(event)
            .map_err(|e| ExecutorError::InvalidCode(format!("Invalid event JSON: {}", e)))?;

        // Base64 encode the event JSON to avoid escaping issues
        let event_b64 = general_purpose::STANDARD.encode(event_json.as_bytes());

        let wrapper = format!(
            r#"#!/usr/bin/env python3
import json
import sys
import traceback
import base64
from function import {handler}

def __wrapper_main__():
    try:
        # Decode event from base64
        event_b64 = "{event_b64}"
        event_json = base64.b64decode(event_b64).decode('utf-8')
        event = json.loads(event_json)
        
        # Create a minimal context object (AWS Lambda-compatible)
        class Context:
            def __init__(self):
                self.function_name = "nanolambda_function"
                self.function_version = "1"
                self.invoked_function_arn = "arn:aws:lambda:us-east-1:000000000000:function:nanolambda"
                self.memory_limit_in_mb = "128"
                self.aws_request_id = "00000000-0000-0000-0000-000000000000"
                self.request_id = self.aws_request_id
                self.log_group_name = "/aws/lambda/nanolambda"
                self.log_stream_name = "2024/01/01/[$LATEST]00000000000000000000000000000000"
                
            def get_remaining_time_in_millis(self):
                return 30000  # 30 seconds
        
        context = Context()
        
        # Execute handler with event and context
        result = {handler}(event, context)
        
        # Output result as JSON
        output = {{
            "success": True,
            "result": result
        }}
        print(json.dumps(output))
        sys.exit(0)
        
    except Exception as e:
        # Capture exception
        error_output = {{
            "success": False,
            "error": str(e),
            "traceback": traceback.format_exc()
        }}
        print(json.dumps(error_output))
        sys.exit(1)

if __name__ == '__main__':
    __wrapper_main__()
"#,
            handler = config.handler,
            event_b64 = event_b64
        );

        Ok(wrapper)
    }

    /// Wait for child process with timeout
    fn wait_with_timeout(
        &self,
        child: std::process::Child,
        timeout: Duration,
    ) -> Result<std::process::Output> {
        use std::sync::mpsc;
        use std::thread;

        let (tx, rx) = mpsc::channel();

        // Spawn a thread to wait for the process
        thread::spawn(move || {
            let output = child.wait_with_output();
            let _ = tx.send(output);
        });

        // Wait for result or timeout
        match rx.recv_timeout(timeout) {
            Ok(Ok(output)) => Ok(output),
            Ok(Err(e)) => Err(ExecutorError::Io(e)),
            Err(_) => Err(ExecutorError::Timeout(timeout)),
        }
    }

    /// Estimate memory usage from process output
    fn estimate_memory_usage(&self, _stdout: &str, _stderr: &str) -> f64 {
        // TODO: Implement real memory tracking using procfs
        // For now, return a placeholder
        64.0
    }

    /// Parse execution output to determine success/failure and extract result
    fn parse_execution_output(
        &self,
        stdout: &str,
        stderr: &str,
        exit_code: Option<i32>,
    ) -> (bool, Option<String>, Option<String>) {
        // Try to parse JSON output
        if let Ok(output) = serde_json::from_str::<serde_json::Value>(stdout)
            && let Some(success) = output.get("success").and_then(|v| v.as_bool())
        {
            if success {
                let result = output.get("result").map(|v| v.to_string());
                return (true, result, None);
            } else {
                let error = output
                    .get("error")
                    .and_then(|v| v.as_str())
                    .unwrap_or("Unknown error")
                    .to_string();
                let traceback = output
                    .get("traceback")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string();

                let full_error = if traceback.is_empty() {
                    error
                } else {
                    format!("{}\n\n{}", error, traceback)
                };

                return (false, None, Some(full_error));
            }
        }

        // If we couldn't parse JSON, check exit code
        let success = exit_code.unwrap_or(-1) == 0;

        if success {
            (true, Some(stdout.to_string()), None)
        } else {
            let error = if !stderr.is_empty() {
                stderr.to_string()
            } else {
                format!("Process exited with code {}", exit_code.unwrap_or(-1))
            };
            (false, None, Some(error))
        }
    }

    /// Get Python version
    pub fn python_version(&self) -> &str {
        &self.python_version
    }

    /// Set whether to keep function directories after execution (for debugging)
    pub fn set_keep_dirs(&mut self, keep: bool) {
        self.keep_dirs = keep;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_executor_creation() {
        let executor = PythonExecutor::new();
        assert!(executor.is_ok(), "Should be able to create executor");

        if let Ok(exec) = executor {
            println!("Python version: {}", exec.python_version());
        }
    }

    #[test]
    fn test_simple_function() {
        let executor = PythonExecutor::new().unwrap();

        let config = FunctionConfig {
            id: 1,
            version: 1,
            name: "test_simple".to_string(),
            code: r#"
def handler(event, context):
    return {"message": "Hello, World!"}
"#
            .to_string(),
            handler: "handler".to_string(),
            environment: HashMap::new(),
            memory_limit_mb: 128,
            timeout_seconds: 5,
            working_dir: None,
        };

        let event = serde_json::json!({});
        let result = executor.execute(config, event).unwrap();

        println!(
            "Result: success={}, result={:?}, error={:?}",
            result.success, result.result, result.error
        );
        assert!(result.success, "Function should execute successfully");
        assert!(
            result.metrics.total_ms < 5000,
            "Should complete within timeout"
        );
        println!("Metrics: {:?}", result.metrics);
    }

    #[test]
    fn test_function_with_event() {
        let executor = PythonExecutor::new().unwrap();

        let config = FunctionConfig {
            id: 2,
            version: 1,
            name: "test_event".to_string(),
            code: r#"
def handler(event, context):
    name = event.get('name', 'World')
    return {"message": f"Hello, {name}!"}
"#
            .to_string(),
            handler: "handler".to_string(),
            environment: HashMap::new(),
            memory_limit_mb: 128,
            timeout_seconds: 5,
            working_dir: None,
        };

        let event = serde_json::json!({"name": "Alice"});
        let result = executor.execute(config, event).unwrap();

        assert!(result.success);
        assert!(result.result.is_some());
        println!("Result: {:?}", result.result);
    }

    #[test]
    fn test_function_error_handling() {
        let executor = PythonExecutor::new().unwrap();

        let config = FunctionConfig {
            id: 3,
            version: 1,
            name: "test_error".to_string(),
            code: r#"
def handler(event, context):
    raise ValueError("Test error")
"#
            .to_string(),
            handler: "handler".to_string(),
            environment: HashMap::new(),
            memory_limit_mb: 128,
            timeout_seconds: 5,
            working_dir: None,
        };

        let event = serde_json::json!({});
        let result = executor.execute(config, event).unwrap();

        assert!(!result.success, "Should fail with error");
        assert!(result.error.is_some(), "Should have error message");
        println!("Error: {:?}", result.error);
    }
}
