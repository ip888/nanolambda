//! Function Composition Engine
//!
//! Chain edge functions together for complex workflows:
//! - Sequential pipelines
//! - Parallel fan-out/fan-in
//! - Conditional branching
//! - Error handling and retry

use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, Instant};
use worker::wasm_bindgen::JsValue;

/// A composable edge function
pub struct EdgeFunction {
    /// Unique function identifier
    pub id: String,
    /// Function name
    pub name: String,
    /// Input schema (JSON Schema format)
    pub input_schema: Option<String>,
    /// Output schema
    pub output_schema: Option<String>,
    /// Timeout for this function
    pub timeout: Duration,
    /// Retry configuration
    pub retry_config: RetryConfig,
}

/// Retry configuration for functions
#[derive(Debug, Clone)]
pub struct RetryConfig {
    /// Maximum retry attempts
    pub max_attempts: u32,
    /// Initial backoff duration
    pub initial_backoff: Duration,
    /// Maximum backoff duration
    pub max_backoff: Duration,
    /// Backoff multiplier
    pub multiplier: f32,
    /// Retryable error codes
    pub retryable_errors: Vec<String>,
}

impl Default for RetryConfig {
    fn default() -> Self {
        Self {
            max_attempts: 3,
            initial_backoff: Duration::from_millis(100),
            max_backoff: Duration::from_secs(10),
            multiplier: 2.0,
            retryable_errors: vec!["TIMEOUT".to_string(), "RATE_LIMITED".to_string()],
        }
    }
}

/// A node in the composition graph
#[derive(Debug, Clone)]
pub enum CompositionNode {
    /// Single function execution
    Function(String),
    /// Sequential execution
    Sequence(Vec<CompositionNode>),
    /// Parallel execution (fan-out)
    Parallel(Vec<CompositionNode>),
    /// Conditional branching
    Conditional {
        condition: String,
        if_true: Box<CompositionNode>,
        if_false: Option<Box<CompositionNode>>,
    },
    /// Transform/map data
    Transform { expression: String },
    /// Wait for external event
    Wait { event: String, timeout: Duration },
}

/// A complete composition (workflow)
pub struct Composition {
    /// Unique composition ID
    pub id: String,
    /// Composition name
    pub name: String,
    /// Root node of the composition
    pub root: CompositionNode,
    /// Global timeout
    pub timeout: Duration,
    /// Error handling strategy
    pub error_strategy: ErrorStrategy,
}

/// Error handling strategy
#[derive(Debug, Clone)]
pub enum ErrorStrategy {
    /// Fail the entire composition
    FailFast,
    /// Continue with partial results
    ContinueOnError,
    /// Use fallback value
    Fallback(String),
    /// Retry the failed step
    Retry(RetryConfig),
}

/// Execution context for a composition
pub struct CompositionContext {
    /// Current data flowing through the composition
    pub data: serde_json::Value,
    /// Variables set during execution
    pub variables: HashMap<String, serde_json::Value>,
    /// Execution trace
    pub trace: Vec<TraceEntry>,
    /// Start time
    pub started_at: Instant,
    /// Execution ID
    pub execution_id: String,
}

/// Entry in the execution trace
#[derive(Debug, Clone)]
pub struct TraceEntry {
    /// Node that was executed
    pub node: String,
    /// Execution status
    pub status: ExecutionStatus,
    /// Duration
    pub duration: Duration,
    /// Input data (truncated)
    pub input_preview: String,
    /// Output data (truncated)
    pub output_preview: String,
    /// Error if any
    pub error: Option<String>,
}

/// Execution status
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExecutionStatus {
    /// Completed successfully
    Success,
    /// Failed with error
    Failed,
    /// Skipped (conditional)
    Skipped,
    /// Retrying
    Retrying,
    /// Timed out
    TimedOut,
}

/// Result of composition execution
pub struct CompositionResult {
    /// Final output data
    pub output: serde_json::Value,
    /// Total execution time
    pub duration: Duration,
    /// Execution trace
    pub trace: Vec<TraceEntry>,
    /// Overall status
    pub status: ExecutionStatus,
    /// Errors encountered
    pub errors: Vec<CompositionError>,
}

/// Composition execution error
#[derive(Debug, Clone)]
pub struct CompositionError {
    /// Node where error occurred
    pub node: String,
    /// Error code
    pub code: String,
    /// Error message
    pub message: String,
    /// Whether the error was handled
    pub handled: bool,
}

/// Composition engine that executes workflows
pub struct CompositionEngine {
    /// Registered functions
    functions: HashMap<String, EdgeFunction>,
    /// Execution statistics
    stats: CompositionStats,
}

/// Composition statistics
#[derive(Debug, Default)]
pub struct CompositionStats {
    /// Total executions
    pub total_executions: AtomicU64,
    /// Successful executions
    pub successful: AtomicU64,
    /// Failed executions
    pub failed: AtomicU64,
    /// Total function invocations
    pub function_invocations: AtomicU64,
    /// Retries performed
    pub retries: AtomicU64,
}

impl CompositionEngine {
    /// Create a new composition engine
    pub fn new() -> Self {
        Self {
            functions: HashMap::new(),
            stats: CompositionStats::default(),
        }
    }

    /// Register a function
    pub fn register_function(&mut self, function: EdgeFunction) {
        self.functions.insert(function.id.clone(), function);
    }

    /// Execute a composition
    pub async fn execute(
        &self,
        composition: &Composition,
        input: serde_json::Value,
    ) -> CompositionResult {
        let start = Instant::now();
        self.stats.total_executions.fetch_add(1, Ordering::Relaxed);

        let mut context = CompositionContext {
            data: input,
            variables: HashMap::new(),
            trace: Vec::new(),
            started_at: start,
            execution_id: generate_execution_id(),
        };

        let result = self
            .execute_node(&composition.root, &mut context, &composition.error_strategy)
            .await;

        let duration = start.elapsed();

        match result {
            Ok(output) => {
                self.stats.successful.fetch_add(1, Ordering::Relaxed);
                CompositionResult {
                    output,
                    duration,
                    trace: context.trace,
                    status: ExecutionStatus::Success,
                    errors: Vec::new(),
                }
            }
            Err(errors) => {
                self.stats.failed.fetch_add(1, Ordering::Relaxed);
                CompositionResult {
                    output: serde_json::Value::Null,
                    duration,
                    trace: context.trace,
                    status: ExecutionStatus::Failed,
                    errors,
                }
            }
        }
    }

    /// Execute a single node
    fn execute_node<'a>(
        &'a self,
        node: &'a CompositionNode,
        context: &'a mut CompositionContext,
        error_strategy: &'a ErrorStrategy,
    ) -> std::pin::Pin<
        Box<
            dyn std::future::Future<Output = Result<serde_json::Value, Vec<CompositionError>>>
                + Send
                + 'a,
        >,
    > {
        Box::pin(async move {
            match node {
                CompositionNode::Function(func_id) => {
                    self.execute_function(func_id, context, error_strategy)
                        .await
                }
                CompositionNode::Sequence(nodes) => {
                    self.execute_sequence(nodes, context, error_strategy).await
                }
                CompositionNode::Parallel(nodes) => {
                    self.execute_parallel(nodes, context, error_strategy).await
                }
                CompositionNode::Conditional {
                    condition,
                    if_true,
                    if_false,
                } => {
                    self.execute_conditional(
                        condition,
                        if_true,
                        if_false.as_deref(),
                        context,
                        error_strategy,
                    )
                    .await
                }
                CompositionNode::Transform { expression } => {
                    self.execute_transform(expression, context).await
                }
                CompositionNode::Wait { event, timeout } => {
                    self.execute_wait(event, *timeout, context).await
                }
            }
        })
    }

    /// Execute a function node
    async fn execute_function(
        &self,
        func_id: &str,
        context: &mut CompositionContext,
        error_strategy: &ErrorStrategy,
    ) -> Result<serde_json::Value, Vec<CompositionError>> {
        let start = Instant::now();
        self.stats
            .function_invocations
            .fetch_add(1, Ordering::Relaxed);

        let function = self.functions.get(func_id).ok_or_else(|| {
            vec![CompositionError {
                node: func_id.to_string(),
                code: "FUNCTION_NOT_FOUND".to_string(),
                message: format!("Function '{}' not registered", func_id),
                handled: false,
            }]
        })?;

        // Simulate function execution
        let result = self.invoke_function(function, &context.data).await;
        let duration = start.elapsed();

        match result {
            Ok(output) => {
                context.trace.push(TraceEntry {
                    node: func_id.to_string(),
                    status: ExecutionStatus::Success,
                    duration,
                    input_preview: truncate_json(&context.data, 100),
                    output_preview: truncate_json(&output, 100),
                    error: None,
                });
                context.data = output.clone();
                Ok(output)
            }
            Err(e) => {
                context.trace.push(TraceEntry {
                    node: func_id.to_string(),
                    status: ExecutionStatus::Failed,
                    duration,
                    input_preview: truncate_json(&context.data, 100),
                    output_preview: "".to_string(),
                    error: Some(e.message.clone()),
                });

                // Handle error based on strategy
                match error_strategy {
                    ErrorStrategy::FailFast => Err(vec![e]),
                    ErrorStrategy::ContinueOnError => Ok(serde_json::Value::Null),
                    ErrorStrategy::Fallback(value) => {
                        serde_json::from_str(value).map_err(|_| vec![e])
                    }
                    ErrorStrategy::Retry(config) => {
                        self.retry_function(function, &context.data, config).await
                    }
                }
            }
        }
    }

    /// Execute a sequence of nodes
    async fn execute_sequence(
        &self,
        nodes: &[CompositionNode],
        context: &mut CompositionContext,
        error_strategy: &ErrorStrategy,
    ) -> Result<serde_json::Value, Vec<CompositionError>> {
        let mut result = context.data.clone();

        for node in nodes {
            result = self.execute_node(node, context, error_strategy).await?;
            context.data = result.clone();
        }

        Ok(result)
    }

    /// Execute nodes in parallel
    async fn execute_parallel(
        &self,
        nodes: &[CompositionNode],
        context: &mut CompositionContext,
        error_strategy: &ErrorStrategy,
    ) -> Result<serde_json::Value, Vec<CompositionError>> {
        // In a real implementation, this would use actual async parallelism
        // For now, execute sequentially but collect results as array
        let mut results = Vec::new();
        let mut errors = Vec::new();

        let input = context.data.clone();

        for node in nodes {
            let mut node_context = CompositionContext {
                data: input.clone(),
                variables: context.variables.clone(),
                trace: Vec::new(),
                started_at: context.started_at,
                execution_id: context.execution_id.clone(),
            };

            match self
                .execute_node(node, &mut node_context, error_strategy)
                .await
            {
                Ok(output) => results.push(output),
                Err(e) => errors.extend(e),
            }

            context.trace.extend(node_context.trace);
        }

        if !errors.is_empty() && matches!(error_strategy, ErrorStrategy::FailFast) {
            return Err(errors);
        }

        Ok(serde_json::Value::Array(results))
    }

    /// Execute conditional node
    async fn execute_conditional(
        &self,
        condition: &str,
        if_true: &CompositionNode,
        if_false: Option<&CompositionNode>,
        context: &mut CompositionContext,
        error_strategy: &ErrorStrategy,
    ) -> Result<serde_json::Value, Vec<CompositionError>> {
        let condition_result = self.evaluate_condition(condition, &context.data);

        if condition_result {
            self.execute_node(if_true, context, error_strategy).await
        } else if let Some(false_branch) = if_false {
            self.execute_node(false_branch, context, error_strategy)
                .await
        } else {
            context.trace.push(TraceEntry {
                node: format!("conditional: {}", condition),
                status: ExecutionStatus::Skipped,
                duration: Duration::ZERO,
                input_preview: truncate_json(&context.data, 100),
                output_preview: "".to_string(),
                error: None,
            });
            Ok(context.data.clone())
        }
    }

    /// Execute transform node
    async fn execute_transform(
        &self,
        expression: &str,
        context: &mut CompositionContext,
    ) -> Result<serde_json::Value, Vec<CompositionError>> {
        let start = Instant::now();

        // Simple JSONPath-like transform (would use proper library in production)
        let result = match expression {
            "$.data" => context
                .data
                .get("data")
                .cloned()
                .unwrap_or(serde_json::Value::Null),
            "$.items" => context
                .data
                .get("items")
                .cloned()
                .unwrap_or(serde_json::Value::Null),
            "$.first" => {
                if let Some(arr) = context.data.as_array() {
                    arr.first().cloned().unwrap_or(serde_json::Value::Null)
                } else {
                    serde_json::Value::Null
                }
            }
            _ => context.data.clone(),
        };

        context.trace.push(TraceEntry {
            node: format!("transform: {}", expression),
            status: ExecutionStatus::Success,
            duration: start.elapsed(),
            input_preview: truncate_json(&context.data, 100),
            output_preview: truncate_json(&result, 100),
            error: None,
        });

        context.data = result.clone();
        Ok(result)
    }

    /// Execute wait node
    async fn execute_wait(
        &self,
        event: &str,
        timeout: Duration,
        context: &mut CompositionContext,
    ) -> Result<serde_json::Value, Vec<CompositionError>> {
        let start = Instant::now();

        // In real implementation, this would wait for an external event
        // For now, just record the wait

        context.trace.push(TraceEntry {
            node: format!("wait: {}", event),
            status: ExecutionStatus::Success,
            duration: start.elapsed(),
            input_preview: truncate_json(&context.data, 100),
            output_preview: truncate_json(&context.data, 100),
            error: None,
        });

        Ok(context.data.clone())
    }

    /// Invoke a function
    async fn invoke_function(
        &self,
        function: &EdgeFunction,
        input: &serde_json::Value,
    ) -> Result<serde_json::Value, CompositionError> {
        // Placeholder - real implementation would call the actual function
        Ok(serde_json::json!({
            "function": function.id,
            "input": input,
            "timestamp": chrono_now(),
        }))
    }

    /// Retry a failed function
    async fn retry_function(
        &self,
        function: &EdgeFunction,
        input: &serde_json::Value,
        config: &RetryConfig,
    ) -> Result<serde_json::Value, Vec<CompositionError>> {
        let mut last_error = None;
        let mut backoff = config.initial_backoff;

        for attempt in 0..config.max_attempts {
            self.stats.retries.fetch_add(1, Ordering::Relaxed);

            match self.invoke_function(function, input).await {
                Ok(output) => return Ok(output),
                Err(e) => {
                    last_error = Some(e);
                    // Would sleep here in real implementation
                    backoff = std::cmp::min(
                        Duration::from_secs_f32(backoff.as_secs_f32() * config.multiplier),
                        config.max_backoff,
                    );
                }
            }
        }

        Err(vec![last_error.unwrap()])
    }

    /// Evaluate a condition expression
    fn evaluate_condition(&self, condition: &str, data: &serde_json::Value) -> bool {
        // Simple condition evaluation (would use proper expression language)
        match condition {
            "$.success" => data
                .get("success")
                .and_then(|v| v.as_bool())
                .unwrap_or(false),
            "$.error" => data.get("error").is_some(),
            "$.items.length > 0" => data
                .get("items")
                .and_then(|v| v.as_array())
                .map(|arr| !arr.is_empty())
                .unwrap_or(false),
            _ => true,
        }
    }

    /// Get engine statistics
    pub fn stats(&self) -> &CompositionStats {
        &self.stats
    }
}

impl Default for CompositionEngine {
    fn default() -> Self {
        Self::new()
    }
}

/// Builder for creating compositions
pub struct CompositionBuilder {
    id: String,
    name: String,
    nodes: Vec<CompositionNode>,
    timeout: Duration,
    error_strategy: ErrorStrategy,
}

impl CompositionBuilder {
    pub fn new(id: impl Into<String>, name: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            name: name.into(),
            nodes: Vec::new(),
            timeout: Duration::from_secs(30),
            error_strategy: ErrorStrategy::FailFast,
        }
    }

    /// Add a function call
    pub fn function(mut self, func_id: impl Into<String>) -> Self {
        self.nodes.push(CompositionNode::Function(func_id.into()));
        self
    }

    /// Add a transform step
    pub fn transform(mut self, expression: impl Into<String>) -> Self {
        self.nodes.push(CompositionNode::Transform {
            expression: expression.into(),
        });
        self
    }

    /// Add parallel execution
    pub fn parallel(mut self, nodes: Vec<CompositionNode>) -> Self {
        self.nodes.push(CompositionNode::Parallel(nodes));
        self
    }

    /// Add conditional
    pub fn conditional(
        mut self,
        condition: impl Into<String>,
        if_true: CompositionNode,
        if_false: Option<CompositionNode>,
    ) -> Self {
        self.nodes.push(CompositionNode::Conditional {
            condition: condition.into(),
            if_true: Box::new(if_true),
            if_false: if_false.map(Box::new),
        });
        self
    }

    /// Set timeout
    pub fn timeout(mut self, timeout: Duration) -> Self {
        self.timeout = timeout;
        self
    }

    /// Set error strategy
    pub fn on_error(mut self, strategy: ErrorStrategy) -> Self {
        self.error_strategy = strategy;
        self
    }

    /// Build the composition
    pub fn build(self) -> Composition {
        Composition {
            id: self.id,
            name: self.name,
            root: if self.nodes.len() == 1 {
                self.nodes.into_iter().next().unwrap()
            } else {
                CompositionNode::Sequence(self.nodes)
            },
            timeout: self.timeout,
            error_strategy: self.error_strategy,
        }
    }
}

// Helper functions

fn generate_execution_id() -> String {
    format!("exec_{}", chrono_now())
}

fn chrono_now() -> u64 {
    use std::time::SystemTime;
    SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap()
        .as_millis() as u64
}

fn truncate_json(value: &serde_json::Value, max_len: usize) -> String {
    let s = value.to_string();
    if s.len() > max_len {
        format!("{}...", &s[..max_len])
    } else {
        s
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_composition_builder() {
        let composition = CompositionBuilder::new("test", "Test Workflow")
            .function("func1")
            .transform("$.data")
            .function("func2")
            .timeout(Duration::from_secs(60))
            .on_error(ErrorStrategy::FailFast)
            .build();

        assert_eq!(composition.id, "test");
        assert_eq!(composition.name, "Test Workflow");
    }

    #[test]
    fn test_retry_config_default() {
        let config = RetryConfig::default();
        assert_eq!(config.max_attempts, 3);
        assert_eq!(config.initial_backoff, Duration::from_millis(100));
    }

    #[test]
    fn test_engine_creation() {
        let engine = CompositionEngine::new();
        assert_eq!(engine.stats.total_executions.load(Ordering::Relaxed), 0);
    }

    #[test]
    fn test_truncate_json() {
        let value = serde_json::json!({"key": "a very long value that should be truncated"});
        let truncated = truncate_json(&value, 20);
        assert!(truncated.ends_with("..."));
        assert!(truncated.len() <= 23); // 20 + "..."
    }
}
