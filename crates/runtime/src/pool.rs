//! Process pool for warm start optimization
//!
//! This module manages a pool of pre-warmed Python processes to reduce cold start latency.
//! Processes are reused across invocations for the same function, providing:
//! - <5ms warm start times (vs ~40ms cold start)
//! - Better resource efficiency
//! - Automatic process lifecycle management

use serde_json::Value;
use std::collections::HashMap;
use std::io::{BufRead, BufReader, Write};
use std::process::{Child, ChildStdin, ChildStdout, Command, Stdio};
use std::sync::{Arc, Mutex};
use std::time::Instant;
use thiserror::Error;

use crate::metrics::ProcessMetrics;

#[derive(Error, Debug)]
pub enum PoolError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Process spawn failed: {0}")]
    SpawnFailed(String),

    #[error("Process communication failed: {0}")]
    CommunicationFailed(String),

    #[error("Process pool full")]
    PoolFull,

    #[error("Process unhealthy: {0}")]
    Unhealthy(String),

    #[error("Timeout waiting for process")]
    Timeout,
}

pub type Result<T> = std::result::Result<T, PoolError>;

/// Result of a warm execution
#[derive(Debug, Clone)]
pub struct WarmExecutionResult {
    /// Whether the execution was successful
    pub success: bool,

    /// Function return value (JSON serialized)
    pub result: String,

    /// Error message if execution failed
    pub error: Option<String>,

    /// Function execution time in milliseconds
    pub execution_ms: u64,

    /// Whether this was a cold start (process creation)
    pub is_cold_start: bool,

    /// Current memory usage in MB
    pub memory_mb: u64,

    /// Peak memory usage in MB
    pub peak_memory_mb: u64,

    /// Average CPU usage percentage
    pub cpu_percent: f64,
}

/// Statistics for a warm process
#[derive(Debug, Clone)]
pub struct ProcessStats {
    /// Number of invocations handled
    pub invocations: u64,

    /// Process age in seconds
    pub age_seconds: u64,

    /// Last invocation timestamp
    pub last_used: Instant,

    /// Average execution time in milliseconds
    pub avg_execution_ms: u64,

    /// Is process currently in use
    pub in_use: bool,
}

/// A warm Python process ready for execution
struct WarmProcess {
    /// The running child process
    child: Child,

    /// Standard input for sending requests
    stdin: ChildStdin,

    /// Buffered reader for receiving responses
    stdout: BufReader<ChildStdout>,

    /// Function code hash (to verify compatibility)
    _code_hash: String,

    /// Statistics
    stats: ProcessStats,

    /// Creation timestamp
    created_at: Instant,

    /// Current process metrics from /proc
    metrics: Option<ProcessMetrics>,

    /// Previous metrics snapshot for delta calculations
    last_metrics: Option<ProcessMetrics>,
}

impl WarmProcess {
    /// Create a new warm process with custom handler name
    fn new(python_path: &str, function_code: &str, handler_name: &str) -> Result<Self> {
        // Create a Python script that stays alive and processes requests
        let runner_script = format!(
            r#"
import sys
import json
import traceback
import time

# Load the function code
function_code = '''
{}
'''

# Execute the function code to define the handler
exec(function_code)

# Get the handler function by name
handler_name = '{}'
if handler_name not in globals():
    sys.stderr.write(f"Error: Handler '{{handler_name}}' not found in function code\n")
    sys.exit(1)
handler_func = globals()[handler_name]

# Main loop: read requests from stdin, execute, write results to stdout
while True:
    try:
        # Read request (one line JSON)
        line = sys.stdin.readline()
        if not line:
            break
            
        request = json.loads(line)
        event = request.get('event', {{}})
        
        # Create a minimal context object (AWS Lambda-compatible)
        class Context:
            def __init__(self):
                self.function_name = "nanolambda_function"
                self.function_version = "1"
                self.invoked_function_arn = "arn:aws:lambda:us-east-1:000000000000:function:nanolambda"
                self.memory_limit_in_mb = "128"
                self.aws_request_id = "00000000-0000-0000-0000-000000000000"
                self.log_group_name = "/aws/lambda/nanolambda"
                self.log_stream_name = "2024/01/01/[$LATEST]00000000000000000000000000000000"
                
            def get_remaining_time_in_millis(self):
                return 30000  # 30 seconds
        
        context = Context()
        
        start_time = time.time()
        
        # Execute the handler with event and context
        try:
            result = handler_func(event, context)
            execution_time = (time.time() - start_time) * 1000
            
            response = {{
                'success': True,
                'result': result,
                'execution_ms': int(execution_time)
            }}
        except Exception as e:
            execution_time = (time.time() - start_time) * 1000
            response = {{
                'success': False,
                'error': str(e),
                'traceback': traceback.format_exc(),
                'execution_ms': int(execution_time)
            }}
        
        # Write response (one line JSON)
        sys.stdout.write(json.dumps(response) + '\n')
        sys.stdout.flush()
        
    except Exception as e:
        # Fatal error in the runner itself
        error_response = {{
            'success': False,
            'error': f'Runner error: {{str(e)}}',
            'fatal': True
        }}
        sys.stdout.write(json.dumps(error_response) + '\n')
        sys.stdout.flush()
        break
"#,
            function_code, handler_name
        );

        // Spawn the Python process
        let mut child = Command::new(python_path)
            .arg("-u") // Unbuffered output
            .arg("-c")
            .arg(&runner_script)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|e| PoolError::SpawnFailed(e.to_string()))?;

        let stdin = child
            .stdin
            .take()
            .ok_or_else(|| PoolError::SpawnFailed("Failed to capture stdin".to_string()))?;

        let stdout = child
            .stdout
            .take()
            .ok_or_else(|| PoolError::SpawnFailed("Failed to capture stdout".to_string()))?;

        // Use blake3 for faster hashing
        let code_hash = blake3::hash(function_code.as_bytes()).to_hex().to_string();

        Ok(Self {
            child,
            stdin,
            stdout: BufReader::new(stdout),
            _code_hash: code_hash,
            stats: ProcessStats {
                invocations: 0,
                age_seconds: 0,
                last_used: Instant::now(),
                avg_execution_ms: 0,
                in_use: false,
            },
            created_at: Instant::now(),
            metrics: None,
            last_metrics: None,
        })
    }

    /// Execute a function invocation on this warm process
    fn invoke(&mut self, event: &Value) -> Result<(bool, String, Option<String>, u64)> {
        // Mark as in use
        self.stats.in_use = true;

        // Collect metrics before execution
        let _ = self.update_metrics();

        // Prepare request
        let request = serde_json::json!({
            "event": event,
            "context": {}
        });

        // Send request to process
        let request_json = serde_json::to_string(&request)
            .map_err(|e| PoolError::CommunicationFailed(e.to_string()))?;

        writeln!(self.stdin, "{}", request_json)
            .map_err(|e| PoolError::CommunicationFailed(e.to_string()))?;

        self.stdin
            .flush()
            .map_err(|e| PoolError::CommunicationFailed(e.to_string()))?;

        // Read response
        let mut response_line = String::new();
        self.stdout
            .read_line(&mut response_line)
            .map_err(|e| PoolError::CommunicationFailed(e.to_string()))?;

        // Parse response
        let response: Value = serde_json::from_str(&response_line)
            .map_err(|e| PoolError::CommunicationFailed(e.to_string()))?;

        let success = response["success"].as_bool().unwrap_or(false);
        let result = response["result"].to_string();
        let error = response["error"].as_str().map(|s| s.to_string());
        let execution_ms = response["execution_ms"].as_u64().unwrap_or(0);

        // Update statistics
        self.stats.invocations += 1;
        self.stats.last_used = Instant::now();
        self.stats.age_seconds = self.created_at.elapsed().as_secs();

        // Update average execution time
        let total_time = self.stats.avg_execution_ms * (self.stats.invocations - 1) + execution_ms;
        self.stats.avg_execution_ms = total_time / self.stats.invocations;

        // Collect metrics after execution
        let _ = self.update_metrics();

        self.stats.in_use = false;

        Ok((success, result, error, execution_ms))
    }

    /// Check if process is still healthy
    fn is_healthy(&mut self) -> bool {
        // Check if process is still running
        match self.child.try_wait() {
            Ok(Some(_)) => false, // Process exited
            Ok(None) => true,     // Process still running
            Err(_) => false,      // Error checking process
        }
    }

    /// Get process statistics
    fn get_stats(&self) -> ProcessStats {
        let mut stats = self.stats.clone();
        stats.age_seconds = self.created_at.elapsed().as_secs();
        stats
    }

    /// Update process metrics from /proc filesystem
    fn update_metrics(&mut self) -> Result<()> {
        let pid = self.child.id();

        match ProcessMetrics::from_pid(pid) {
            Ok(new_metrics) => {
                self.last_metrics = self.metrics.take();
                self.metrics = Some(new_metrics);
                Ok(())
            }
            Err(e) => Err(PoolError::CommunicationFailed(format!(
                "Failed to collect metrics: {}",
                e
            ))),
        }
    }

    /// Get current memory usage in MB (RSS)
    fn get_memory_mb(&self) -> u64 {
        self.metrics
            .as_ref()
            .map(|m| m.rss_bytes / 1024 / 1024)
            .unwrap_or(0)
    }

    /// Get peak memory usage in MB
    fn get_peak_memory_mb(&self) -> u64 {
        self.metrics
            .as_ref()
            .map(|m| m.rss_peak_bytes / 1024 / 1024)
            .unwrap_or(0)
    }

    /// Get CPU usage percentage
    fn get_cpu_percent(&self) -> f64 {
        match (&self.metrics, &self.last_metrics) {
            (Some(current), Some(previous)) => current.cpu_percent(previous),
            _ => 0.0,
        }
    }
}

impl Drop for WarmProcess {
    fn drop(&mut self) {
        // Kill the process when dropped
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

/// Process pool manager
pub struct ProcessPool {
    /// Python interpreter path
    python_path: String,

    /// Pool of warm processes indexed by (function_id, version) composite key
    processes: Arc<Mutex<HashMap<String, WarmProcess>>>,

    /// Maximum pool size
    max_size: usize,

    /// Maximum process age before recycling (seconds)
    max_age_seconds: u64,

    /// Maximum invocations per process before recycling
    max_invocations: u64,
}

impl ProcessPool {
    /// Create a new process pool
    pub fn new(python_path: String) -> Self {
        Self {
            python_path,
            processes: Arc::new(Mutex::new(HashMap::new())),
            max_size: 100,
            max_age_seconds: 3600, // 1 hour
            max_invocations: 1000,
        }
    }

    /// Create cache key from function_id and version
    fn make_cache_key(function_id: i64, version: i64) -> String {
        format!("{}:{}", function_id, version)
    }

    /// Get or create a warm process for a function version
    fn get_or_create(
        &self,
        function_id: i64,
        version: i64,
        function_code: &str,
        handler_name: &str,
    ) -> Result<()> {
        let cache_key = Self::make_cache_key(function_id, version);
        let mut processes = self
            .processes
            .lock()
            .map_err(|e| PoolError::CommunicationFailed(format!("Mutex poisoned: {}", e)))?;

        // Check if we already have a process for this function version
        if let Some(process) = processes.get_mut(&cache_key) {
            // Check if process is still healthy and not too old
            if process.is_healthy()
                && process.stats.age_seconds < self.max_age_seconds
                && process.stats.invocations < self.max_invocations
            {
                return Ok(());
            } else {
                // Remove unhealthy or old process
                processes.remove(&cache_key);
            }
        }

        // Check pool size
        if processes.len() >= self.max_size {
            return Err(PoolError::PoolFull);
        }

        // Create new warm process
        let process = WarmProcess::new(&self.python_path, function_code, handler_name)?;
        processes.insert(cache_key, process);

        Ok(())
    }

    /// Execute a function using a warm process
    pub fn execute_warm(
        &self,
        function_id: i64,
        version: i64,
        function_code: &str,
        handler_name: &str,
        event: &Value,
    ) -> Result<WarmExecutionResult> {
        let cache_key = Self::make_cache_key(function_id, version);

        // Ensure we have a warm process
        let is_cold_start = {
            let processes = self
                .processes
                .lock()
                .map_err(|e| PoolError::CommunicationFailed(format!("Mutex poisoned: {}", e)))?;
            !processes.contains_key(&cache_key)
        };

        self.get_or_create(function_id, version, function_code, handler_name)?;

        // Execute on the warm process
        let mut processes = self
            .processes
            .lock()
            .map_err(|e| PoolError::CommunicationFailed(format!("Mutex poisoned: {}", e)))?;
        let process = processes
            .get_mut(&cache_key)
            .ok_or(PoolError::CommunicationFailed(
                "Process disappeared".to_string(),
            ))?;

        let (success, result, error, execution_ms) = process.invoke(event)?;

        // Get memory and CPU metrics
        let memory_mb = process.get_memory_mb();
        let peak_memory_mb = process.get_peak_memory_mb();
        let cpu_percent = process.get_cpu_percent();

        Ok(WarmExecutionResult {
            success,
            result,
            error,
            execution_ms,
            is_cold_start,
            memory_mb,
            peak_memory_mb,
            cpu_percent,
        })
    }

    /// Get statistics for all processes
    pub fn get_pool_stats(&self) -> HashMap<String, ProcessStats> {
        let processes = self.processes.lock().unwrap_or_else(|e| e.into_inner());
        processes
            .iter()
            .map(|(name, process)| (name.clone(), process.get_stats()))
            .collect()
    }

    /// Clean up old or unhealthy processes
    pub fn cleanup(&self) {
        let mut processes = self.processes.lock().unwrap_or_else(|e| e.into_inner());

        // Remove unhealthy or old processes
        processes.retain(|_, process| {
            process.is_healthy()
                && process.stats.age_seconds < self.max_age_seconds
                && process.stats.invocations < self.max_invocations
        });
    }

    /// Get pool size
    pub fn size(&self) -> usize {
        self.processes
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_process_pool_creation() {
        let pool = ProcessPool::new("python3".to_string());
        assert_eq!(pool.size(), 0);
    }

    #[test]
    fn test_warm_execution() {
        let pool = ProcessPool::new("python3".to_string());

        let function_code = r#"
def handler(event, context):
    name = event.get('name', 'World')
    return {'message': f'Hello, {name}!'}
"#;

        let event = serde_json::json!({"name": "Test"});

        // First invocation (cold start) - function_id: 1, version: 1
        let result1 = pool.execute_warm(1, 1, function_code, "handler", &event);
        assert!(result1.is_ok());
        let warm_result1 = result1.unwrap();
        assert!(warm_result1.success);
        assert!(warm_result1.is_cold_start); // First call should be cold start

        // Second invocation (warm start) - same function_id and version
        let result2 = pool.execute_warm(1, 1, function_code, "handler", &event);
        assert!(result2.is_ok());
        let warm_result2 = result2.unwrap();
        assert!(warm_result2.success);
        assert!(!warm_result2.is_cold_start); // Second call should be warm start

        // Third invocation with different version - should be cold start
        let result3 = pool.execute_warm(1, 2, function_code, "handler", &event);
        assert!(result3.is_ok());
        let warm_result3 = result3.unwrap();
        assert!(warm_result3.success);
        assert!(warm_result3.is_cold_start); // Different version = cold start
    }
}
