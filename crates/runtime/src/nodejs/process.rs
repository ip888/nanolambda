//! Node.js process management
//!
//! Handles spawning and managing individual Node.js processes for function execution.

use super::NodeError;
use crate::metrics::ProcessMetrics;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::io::{BufRead, BufReader, Write};
use std::process::{Child, ChildStdin, ChildStdout, Command, Stdio};
use std::time::{Duration, Instant};

/// Node.js wrapper script that handles function execution
/// This script is embedded as a string and executed by Node.js
const NODE_WRAPPER_SCRIPT: &str = r#"
const readline = require('readline');

// Read function code from process.argv[1] (when using -e, additional args start at index 1)
const functionCode = process.argv[1];

// Load the handler function
let handler;
try {
    // Create a module-like environment
    const module = { exports: {} };
    let exports = module.exports;
    
    // Evaluate the function code in a way that captures exports
    const wrappedCode = `(function(exports, module) { ${functionCode} })(exports, module);`;
    eval(wrappedCode);
    
    // Get the handler
    handler = module.exports.handler || exports.handler;
    
    if (!handler || typeof handler !== 'function') {
        process.stdout.write(JSON.stringify({
            success: false,
            error: 'Handler function not found or not a function. Available: ' + Object.keys(module.exports).join(', '),
            stack: ''
        }) + '\n');
        process.exit(1);
    }
} catch (error) {
    process.stdout.write(JSON.stringify({
        success: false,
        error: error.message,
        stack: error.stack || ''
    }) + '\n');
    process.exit(1);
}

// Send ready signal
process.stdout.write(JSON.stringify({ ready: true }) + '\n');

// Create readline interface for IPC (no output to avoid conflicts)
const rl = readline.createInterface({
    input: process.stdin,
    terminal: false
});

// Process incoming requests
rl.on('line', async (line) => {
    try {
        const request = JSON.parse(line);
        const start = Date.now();
        
        // Execute the handler
        const result = await Promise.resolve(handler(request.event, request.context || {}));
        const executionMs = Date.now() - start;
        
        // Send success response
        const response = {
            success: true,
            result: JSON.stringify(result),
            execution_ms: executionMs
        };
        
        process.stdout.write(JSON.stringify(response) + '\n');
    } catch (error) {
        // Send error response
        const response = {
            success: false,
            error: error.message || String(error),
            stack: error.stack || '',
            execution_ms: 0
        };
        
        process.stdout.write(JSON.stringify(response) + '\n');
    }
});

// Handle errors
rl.on('error', (error) => {
    process.stdout.write(JSON.stringify({
        success: false,
        error: error.message,
        stack: error.stack || ''
    }) + '\n');
});
"#;

/// Process statistics
#[derive(Debug, Clone, Default)]
pub struct ProcessStats {
    pub invocations: u64,
    pub total_execution_ms: u64,
    pub errors: u64,
}

/// Request sent to Node.js process
#[derive(Debug, Serialize)]
struct NodeRequest {
    event: Value,
    context: Value,
}

/// Response received from Node.js process
#[derive(Debug, Deserialize)]
#[serde(untagged)]
enum NodeResponse {
    Ready {
        ready: bool,
    },
    Success {
        success: bool,
        result: String,
        execution_ms: u64,
    },
    Error {
        success: bool,
        error: String,
        stack: String,
        #[serde(default)]
        execution_ms: u64,
    },
}

/// A running Node.js process
pub struct NodeProcess {
    child: Child,
    stdin: ChildStdin,
    stdout: BufReader<ChildStdout>,
    code_hash: String,
    stats: ProcessStats,
    created_at: Instant,
    metrics: Option<ProcessMetrics>,
    last_metrics: Option<ProcessMetrics>,
}

impl NodeProcess {
    /// Spawn a new Node.js process with the given function code
    pub fn new(node_path: &str, function_code: &str, code_hash: String) -> Result<Self, NodeError> {
        // Spawn the Node.js process
        let mut child = Command::new(node_path)
            .arg("-e")
            .arg(NODE_WRAPPER_SCRIPT)
            .arg(function_code)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
            .map_err(|e| NodeError::SpawnFailed(e.to_string()))?;

        let stdin = child.stdin.take().ok_or_else(|| {
            NodeError::SpawnFailed("Failed to capture stdin".to_string())
        })?;

        let stdout = child.stdout.take().ok_or_else(|| {
            NodeError::SpawnFailed("Failed to capture stdout".to_string())
        })?;

        let mut process = NodeProcess {
            child,
            stdin,
            stdout: BufReader::new(stdout),
            code_hash,
            stats: ProcessStats::default(),
            created_at: Instant::now(),
            metrics: None,
            last_metrics: None,
        };

        // Wait for ready signal
        process.wait_for_ready()?;

        // Initialize metrics
        let pid = process.child.id();
        if let Ok(metrics) = ProcessMetrics::from_pid(pid) {
            process.metrics = Some(metrics);
        }

        Ok(process)
    }

    /// Wait for the process to signal it's ready
    fn wait_for_ready(&mut self) -> Result<(), NodeError> {
        let mut line = String::new();
        self.stdout
            .read_line(&mut line)
            .map_err(|e| NodeError::IpcError(format!("Failed to read ready signal: {}", e)))?;

        let response: NodeResponse = serde_json::from_str(&line)
            .map_err(|e| NodeError::IpcError(format!("Invalid ready response: {}", e)))?;

        match response {
            NodeResponse::Ready { ready: true } => Ok(()),
            NodeResponse::Error { error, stack, .. } => {
                Err(NodeError::ProcessError(format!("{}\n{}", error, stack)))
            }
            _ => Err(NodeError::IpcError("Unexpected ready response".to_string())),
        }
    }

    /// Invoke the function with the given event
    pub fn invoke(&mut self, event: &Value) -> Result<InvocationResult, NodeError> {
        // Update metrics before invocation
        self.update_metrics();

        let start = Instant::now();

        // Send request
        let request = NodeRequest {
            event: event.clone(),
            context: Value::Object(serde_json::Map::new()),
        };

        let request_json = serde_json::to_string(&request)
            .map_err(|e| NodeError::IpcError(format!("Failed to serialize request: {}", e)))?;

        writeln!(self.stdin, "{}", request_json)
            .map_err(|e| NodeError::IpcError(format!("Failed to write request: {}", e)))?;

        self.stdin
            .flush()
            .map_err(|e| NodeError::IpcError(format!("Failed to flush stdin: {}", e)))?;

        // Read response
        let mut line = String::new();
        self.stdout
            .read_line(&mut line)
            .map_err(|e| NodeError::IpcError(format!("Failed to read response: {}", e)))?;

        let response: NodeResponse = serde_json::from_str(&line)
            .map_err(|e| NodeError::IpcError(format!("Invalid response: {}", e)))?;

        // Update metrics after invocation
        self.update_metrics();

        let total_duration = start.elapsed();

        // Process response
        match response {
            NodeResponse::Success {
                result,
                execution_ms,
                ..
            } => {
                self.stats.invocations += 1;
                self.stats.total_execution_ms += execution_ms;

                let result_value: Value = serde_json::from_str(&result)
                    .map_err(|e| NodeError::IpcError(format!("Failed to parse result: {}", e)))?;

                let cpu_percent = match (&self.metrics, &self.last_metrics) {
                    (Some(current), Some(previous)) => Some(current.cpu_percent(previous)),
                    _ => None,
                };

                Ok(InvocationResult {
                    success: true,
                    result: Some(result_value),
                    error: None,
                    execution_ms,
                    total_duration_ms: total_duration.as_millis() as u64,
                    memory_used_mb: self.metrics.as_ref().map(|m| m.memory_mb()),
                    cpu_percent,
                })
            }
            NodeResponse::Error {
                error,
                stack,
                execution_ms,
                ..
            } => {
                self.stats.errors += 1;

                let cpu_percent = match (&self.metrics, &self.last_metrics) {
                    (Some(current), Some(previous)) => Some(current.cpu_percent(previous)),
                    _ => None,
                };

                Ok(InvocationResult {
                    success: false,
                    result: None,
                    error: Some(format!("{}\n{}", error, stack)),
                    execution_ms,
                    total_duration_ms: total_duration.as_millis() as u64,
                    memory_used_mb: self.metrics.as_ref().map(|m| m.memory_mb()),
                    cpu_percent,
                })
            }
            _ => Err(NodeError::IpcError("Unexpected response type".to_string())),
        }
    }

    /// Check if the process is still healthy
    pub fn is_healthy(&mut self) -> bool {
        match self.child.try_wait() {
            Ok(Some(_status)) => false, // Process has exited
            Ok(None) => true,            // Process is still running
            Err(_) => false,             // Error checking process status
        }
    }

    /// Update process metrics
    fn update_metrics(&mut self) {
        let pid = self.child.id();
        if let Ok(new_metrics) = ProcessMetrics::from_pid(pid) {
            self.last_metrics = self.metrics.clone();
            self.metrics = Some(new_metrics);
        }
    }

    /// Get the code hash for this process
    pub fn code_hash(&self) -> &str {
        &self.code_hash
    }

    /// Get process statistics
    pub fn stats(&self) -> &ProcessStats {
        &self.stats
    }

    /// Get the process ID
    pub fn pid(&self) -> u32 {
        self.child.id()
    }

    /// Get the process age
    pub fn age(&self) -> Duration {
        self.created_at.elapsed()
    }

    /// Get current metrics
    pub fn metrics(&self) -> Option<&ProcessMetrics> {
        self.metrics.as_ref()
    }
}

impl Drop for NodeProcess {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

/// Result of a function invocation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InvocationResult {
    pub success: bool,
    pub result: Option<Value>,
    pub error: Option<String>,
    pub execution_ms: u64,
    pub total_duration_ms: u64,
    pub memory_used_mb: Option<f64>,
    pub cpu_percent: Option<f64>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::nodejs::detect_nodejs;

    fn get_node_path() -> Option<String> {
        detect_nodejs().ok().map(|(path, _)| path.to_string_lossy().to_string())
    }

    #[test]
    fn test_simple_function() {
        let node_path = match get_node_path() {
            Some(path) => path,
            None => {
                println!("Node.js not found - skipping test");
                return;
            }
        };

        let function_code = r#"
            exports.handler = (event) => {
                return { message: `Hello, ${event.name}!` };
            };
        "#;

        let mut process = NodeProcess::new(
            &node_path,
            function_code,
            "test_hash".to_string(),
        ).unwrap();

        let event = serde_json::json!({ "name": "World" });
        let result = process.invoke(&event).unwrap();

        assert!(result.success);
        assert!(result.error.is_none());
        assert_eq!(
            result.result.unwrap(),
            serde_json::json!({ "message": "Hello, World!" })
        );
    }

    #[test]
    fn test_async_function() {
        let node_path = match get_node_path() {
            Some(path) => path,
            None => {
                println!("Node.js not found - skipping test");
                return;
            }
        };

        let function_code = r#"
            exports.handler = async (event) => {
                await new Promise(resolve => setTimeout(resolve, 10));
                return { result: event.value * 2 };
            };
        "#;

        let mut process = NodeProcess::new(
            &node_path,
            function_code,
            "test_hash".to_string(),
        ).unwrap();

        let event = serde_json::json!({ "value": 21 });
        let result = process.invoke(&event).unwrap();

        assert!(result.success);
        assert_eq!(
            result.result.unwrap(),
            serde_json::json!({ "result": 42 })
        );
    }

    #[test]
    fn test_error_handling() {
        let node_path = match get_node_path() {
            Some(path) => path,
            None => {
                println!("Node.js not found - skipping test");
                return;
            }
        };

        let function_code = r#"
            exports.handler = (event) => {
                throw new Error('Test error');
            };
        "#;

        let mut process = NodeProcess::new(
            &node_path,
            function_code,
            "test_hash".to_string(),
        ).unwrap();

        let event = serde_json::json!({});
        let result = process.invoke(&event).unwrap();

        assert!(!result.success);
        assert!(result.error.is_some());
        assert!(result.error.unwrap().contains("Test error"));
    }

    #[test]
    fn test_multiple_invocations() {
        let node_path = match get_node_path() {
            Some(path) => path,
            None => {
                println!("Node.js not found - skipping test");
                return;
            }
        };

        let function_code = r#"
            let counter = 0;
            exports.handler = (event) => {
                counter++;
                return { counter, input: event.value };
            };
        "#;

        let mut process = NodeProcess::new(
            &node_path,
            function_code,
            "test_hash".to_string(),
        ).unwrap();

        // First invocation
        let result1 = process.invoke(&serde_json::json!({ "value": "first" })).unwrap();
        assert!(result1.success);
        assert_eq!(
            result1.result.unwrap(),
            serde_json::json!({ "counter": 1, "input": "first" })
        );

        // Second invocation (counter should increment)
        let result2 = process.invoke(&serde_json::json!({ "value": "second" })).unwrap();
        assert!(result2.success);
        assert_eq!(
            result2.result.unwrap(),
            serde_json::json!({ "counter": 2, "input": "second" })
        );

        // Check stats
        assert_eq!(process.stats().invocations, 2);
    }
}
