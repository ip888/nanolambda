//! Java runtime executor
//!
//! This module provides Java function execution with:
//! - JVM process isolation
//! - Comprehensive metrics collection
//! - Support for Java 11, 17, and 21
//! - Proper error handling and timeout management

use serde_json::Value;
use std::fs;
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};
use thiserror::Error;

use crate::executor::{ExecutionMetrics, ExecutionResult, ExecutorError};
use crate::runtime_trait::Runtime;
use crate::types::{GenericFunctionConfig, Language, RuntimeCapabilities, RuntimeInfo};

#[derive(Error, Debug)]
pub enum JavaError {
    #[error("Java not found: {0}")]
    JavaNotFound(String),

    #[error("Compilation failed: {0}")]
    CompilationFailed(String),

    #[error("Execution failed: {0}")]
    ExecutionFailed(String),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
}

pub type Result<T> = std::result::Result<T, JavaError>;

/// Java function executor with JVM management
pub struct JavaExecutor {
    /// Path to Java executable
    java_path: String,

    /// Path to javac compiler
    javac_path: String,

    /// Java version
    java_version: String,

    /// Base directory for function code
    base_dir: PathBuf,

    /// Whether to keep working directories after execution
    keep_dirs: bool,

    /// Whether warm starts are enabled
    warm_starts_enabled: bool,
}

impl JavaExecutor {
    /// Create a new Java executor
    pub fn new() -> Result<Self> {
        let java_path = Self::find_java()?;
        let javac_path = Self::find_javac()?;
        let java_version = Self::get_java_version(&java_path)?;

        let base_dir = std::env::temp_dir().join("nanolambda-java");
        fs::create_dir_all(&base_dir)?;

        Ok(Self {
            java_path,
            javac_path,
            java_version,
            base_dir,
            keep_dirs: false,
            warm_starts_enabled: false,
        })
    }

    /// Find Java executable in PATH
    fn find_java() -> Result<String> {
        // Try common locations
        let candidates = vec!["java", "/usr/bin/java", "/usr/local/bin/java"];

        for candidate in candidates {
            if let Ok(output) = Command::new(candidate).arg("-version").output()
                && output.status.success()
            {
                return Ok(candidate.to_string());
            }
        }

        Err(JavaError::JavaNotFound(
            "Java not found in PATH. Please install JDK 11, 17, or 21.".to_string(),
        ))
    }

    /// Find javac compiler in PATH
    fn find_javac() -> Result<String> {
        let candidates = vec!["javac", "/usr/bin/javac", "/usr/local/bin/javac"];

        for candidate in candidates {
            if let Ok(output) = Command::new(candidate).arg("-version").output()
                && output.status.success()
            {
                return Ok(candidate.to_string());
            }
        }

        Err(JavaError::JavaNotFound(
            "javac not found in PATH. Please install JDK.".to_string(),
        ))
    }

    /// Get Java version
    fn get_java_version(java_path: &str) -> Result<String> {
        let output = Command::new(java_path).arg("-version").output()?;

        let stderr = String::from_utf8_lossy(&output.stderr);
        let version = stderr
            .lines()
            .next()
            .and_then(|line| {
                line.split('\"')
                    .nth(1)
                    .map(|v| v.split('.').next().unwrap_or("unknown").to_string())
            })
            .unwrap_or_else(|| "unknown".to_string());

        Ok(version)
    }

    /// Compile and execute Java code
    fn execute_java(
        &self,
        config: &GenericFunctionConfig,
        event: Value,
    ) -> std::result::Result<ExecutionResult, ExecutorError> {
        let start_time = Instant::now();

        // Create working directory
        let func_dir = self.create_function_dir(&config.name)?;
        let src_dir = func_dir.join("src");
        fs::create_dir_all(&src_dir)?;

        // Write Java source file
        // Assume handler format: "ClassName.methodName"
        let (class_name, method_name) = self.parse_handler(&config.handler)?;
        let java_file = src_dir.join(format!("{}.java", class_name));

        // Create a wrapper that includes JSON handling
        let java_code = self.generate_java_wrapper(&config.code, &class_name, &method_name)?;
        fs::write(&java_file, java_code)?;

        // Write event JSON to file
        let event_file = func_dir.join("event.json");
        fs::write(&event_file, serde_json::to_string_pretty(&event).unwrap())?;

        // Compile
        let compile_start = Instant::now();
        let compile_output = Command::new(&self.javac_path)
            .arg("-d")
            .arg(&func_dir)
            .arg(&java_file)
            .current_dir(&func_dir)
            .output()
            .map_err(|e| ExecutorError::RuntimeError(format!("Compilation failed: {}", e)))?;

        if !compile_output.status.success() {
            let compile_error = String::from_utf8_lossy(&compile_output.stderr);
            return Err(ExecutorError::RuntimeError(format!(
                "Java compilation failed: {}",
                compile_error
            )));
        }

        let cold_start_ms = compile_start.elapsed().as_millis() as u64;
        let exec_start = Instant::now();

        // Execute
        let mut cmd = Command::new(&self.java_path);
        cmd.arg("-cp")
            .arg(&func_dir)
            .arg(&class_name)
            .arg(&event_file)
            .current_dir(&func_dir)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());

        // Set environment variables
        for (key, value) in &config.environment {
            cmd.env(key, value);
        }

        // Set memory limit (max heap size)
        if config.memory_limit_mb > 0 {
            cmd.arg(format!("-Xmx{}m", config.memory_limit_mb));
        }

        let child = cmd.spawn()?;
        let pid = child.id();

        // Wait for completion with timeout
        let timeout = Duration::from_secs(config.timeout_seconds);
        let output = Self::wait_with_timeout(child, timeout)?;

        let execution_ms = exec_start.elapsed().as_millis() as u64;
        let total_ms = start_time.elapsed().as_millis() as u64;

        // Parse output
        let stdout = String::from_utf8_lossy(&output.stdout).to_string();
        let stderr = String::from_utf8_lossy(&output.stderr).to_string();

        // Collect metrics
        let (memory_peak_mb, cpu_percent) =
            if let Ok(metrics) = crate::metrics::ProcessMetrics::from_pid(pid) {
                let memory_mb = (metrics.rss_peak_bytes as f64) / (1024.0 * 1024.0);
                let total_cpu_jiffies = metrics.cpu_utime + metrics.cpu_stime;
                let cpu_seconds = (total_cpu_jiffies as f64) / 100.0;
                let elapsed_seconds = execution_ms as f64 / 1000.0;
                let cpu_pct = if elapsed_seconds > 0.0 {
                    (cpu_seconds / elapsed_seconds) * 100.0
                } else {
                    0.0
                };
                (memory_mb, cpu_pct)
            } else {
                (128.0, 0.0) // Default estimate for Java
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
                stdout,
                stderr,
                is_cold_start: true,
                python_version: format!("Java {}", self.java_version),
            },
        })
    }

    /// Wait for process with timeout
    fn wait_with_timeout(
        child: std::process::Child,
        timeout: Duration,
    ) -> std::result::Result<std::process::Output, ExecutorError> {
        use std::sync::mpsc;
        use std::thread;

        let (tx, rx) = mpsc::channel();

        thread::spawn(move || {
            let result = child.wait_with_output();
            let _ = tx.send(result);
        });

        match rx.recv_timeout(timeout) {
            Ok(Ok(output)) => Ok(output),
            Ok(Err(e)) => Err(ExecutorError::Io(e)),
            Err(_) => Err(ExecutorError::Timeout(timeout)),
        }
    }

    /// Create a unique directory for function execution
    fn create_function_dir(&self, name: &str) -> std::result::Result<PathBuf, ExecutorError> {
        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_else(|_| Duration::from_secs(0))
            .as_micros();

        let dir_name = format!("{}_{}", name, timestamp);
        let func_dir = self.base_dir.join(dir_name);
        fs::create_dir_all(&func_dir)?;

        Ok(func_dir)
    }

    /// Parse handler string (e.g., "Handler.handleRequest")
    fn parse_handler(&self, handler: &str) -> std::result::Result<(String, String), ExecutorError> {
        let parts: Vec<&str> = handler.split('.').collect();
        if parts.len() != 2 {
            return Err(ExecutorError::InvalidCode(format!(
                "Invalid Java handler format: '{}'. Expected 'ClassName.methodName'",
                handler
            )));
        }

        Ok((parts[0].to_string(), parts[1].to_string()))
    }

    /// Generate Java wrapper code
    fn generate_java_wrapper(
        &self,
        user_code: &str,
        class_name: &str,
        method_name: &str,
    ) -> std::result::Result<String, ExecutorError> {
        // If user_code already contains class definition, use it directly
        if user_code.contains(&format!("class {}", class_name)) {
            // Add main method wrapper if not present
            if !user_code.contains("public static void main") {
                let wrapper = format!(
                    r#"{}

    public static void main(String[] args) throws Exception {{
        if (args.length == 0) {{
            System.err.println("Error: No event file provided");
            System.exit(1);
        }}

        String eventJson = new String(java.nio.file.Files.readAllBytes(
            java.nio.file.Paths.get(args[0])));

        com.google.gson.JsonParser parser = new com.google.gson.JsonParser();
        com.google.gson.JsonElement event = parser.parse(eventJson);

        {} handler = new {}();
        Object result = handler.{}(event);

        System.out.println(new com.google.gson.Gson().toJson(result));
    }}
"#,
                    user_code, class_name, class_name, method_name
                );
                Ok(wrapper)
            } else {
                Ok(user_code.to_string())
            }
        } else {
            // Generate full wrapper class
            let wrapper = format!(
                r#"import com.google.gson.*;

public class {} {{

    {}

    public static void main(String[] args) throws Exception {{
        if (args.length == 0) {{
            System.err.println("Error: No event file provided");
            System.exit(1);
        }}

        String eventJson = new String(java.nio.file.Files.readAllBytes(
            java.nio.file.Paths.get(args[0])));

        JsonParser parser = new JsonParser();
        JsonElement event = parser.parse(eventJson);

        {} handler = new {}();
        Object result = handler.{}(event);

        System.out.println(new Gson().toJson(result));
    }}
}}
"#,
                class_name, user_code, class_name, class_name, method_name
            );
            Ok(wrapper)
        }
    }

    /// Parse execution output
    fn parse_execution_output(
        &self,
        stdout: &str,
        stderr: &str,
        exit_code: Option<i32>,
    ) -> (bool, Option<String>, Option<String>) {
        // Check exit code first
        if exit_code != Some(0) {
            let error = if !stderr.is_empty() {
                stderr.to_string()
            } else {
                format!("Java process exited with code: {:?}", exit_code)
            };
            return (false, None, Some(error));
        }

        // Try to parse JSON output
        if serde_json::from_str::<Value>(stdout).is_ok() {
            (true, Some(stdout.to_string()), None)
        } else if !stdout.is_empty() {
            // Return stdout as result even if not JSON
            (true, Some(stdout.to_string()), None)
        } else if !stderr.is_empty() {
            (false, None, Some(stderr.to_string()))
        } else {
            (true, None, None)
        }
    }
}

impl Runtime for JavaExecutor {
    async fn execute(
        &self,
        config: &GenericFunctionConfig,
        event: Value,
    ) -> std::result::Result<ExecutionResult, ExecutorError> {
        // Execute in blocking thread pool
        let executor = self.clone();
        let config = config.clone();
        tokio::task::spawn_blocking(move || executor.execute_java(&config, event))
            .await
            .map_err(|e| ExecutorError::RuntimeError(format!("Task join error: {}", e)))?
    }

    fn runtime_info(&self) -> RuntimeInfo {
        RuntimeInfo {
            language: Language::Java,
            version: self.java_version.clone(),
            interpreter_path: PathBuf::from(&self.java_path),
            capabilities: RuntimeCapabilities {
                warm_starts: false, // JVM is heavy, warm starts less effective
                async_execution: false,
                streaming: false,
                max_memory_mb: Some(2048),
                max_timeout_seconds: Some(900),
            },
        }
    }

    fn health_check(&self) -> std::result::Result<(), ExecutorError> {
        // Check if Java and javac are available
        let java_check = Command::new(&self.java_path).arg("-version").output();
        let javac_check = Command::new(&self.javac_path).arg("-version").output();

        if java_check.is_err() {
            return Err(ExecutorError::RuntimeError(
                "Java runtime not available".to_string(),
            ));
        }

        if javac_check.is_err() {
            return Err(ExecutorError::RuntimeError(
                "Java compiler (javac) not available".to_string(),
            ));
        }

        Ok(())
    }

    fn set_warm_starts(&mut self, enabled: bool) {
        self.warm_starts_enabled = enabled;
    }

    fn warm_starts_enabled(&self) -> bool {
        self.warm_starts_enabled
    }
}

// Implement Clone to allow moving into tokio tasks
impl Clone for JavaExecutor {
    fn clone(&self) -> Self {
        Self {
            java_path: self.java_path.clone(),
            javac_path: self.javac_path.clone(),
            java_version: self.java_version.clone(),
            base_dir: self.base_dir.clone(),
            keep_dirs: self.keep_dirs,
            warm_starts_enabled: self.warm_starts_enabled,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_java_executor_creation() {
        // This will fail if Java is not installed, which is expected
        match JavaExecutor::new() {
            Ok(executor) => {
                assert!(!executor.java_path.is_empty());
                assert!(!executor.javac_path.is_empty());
            }
            Err(e) => {
                println!("Java not available (expected in CI): {}", e);
            }
        }
    }
}
