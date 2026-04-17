//! Server-Sent Events (SSE) handlers for streaming function output
//!
//! Provides real-time streaming of function execution results using SSE.
//! This is useful for long-running functions that produce output over time.
//!
//! # Example Client Usage
//!
//! ```javascript
//! const eventSource = new EventSource('/v1/functions/my-func/stream');
//! eventSource.onmessage = (event) => {
//!     const data = JSON.parse(event.data);
//!     console.log('Received:', data);
//! };
//! eventSource.onerror = (err) => {
//!     console.error('Stream error:', err);
//! };
//! ```

use axum::{
    extract::{Path, State},
    response::sse::{Event, KeepAlive, Sse},
    Json,
};
use futures::stream::Stream;
use futures::StreamExt;
use serde::{Deserialize, Serialize};
use std::convert::Infallible;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::mpsc;
use tokio_stream::wrappers::ReceiverStream;
use tracing::{debug, info};

use crate::ApiServer;

/// SSE stream event types
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum StreamEvent {
    /// Function execution started
    Start {
        request_id: String,
        function_name: String,
        timestamp: String,
    },
    /// Log output from function
    Log {
        request_id: String,
        level: String,
        message: String,
        timestamp: String,
    },
    /// Partial output chunk
    Output {
        request_id: String,
        chunk: serde_json::Value,
        index: u32,
    },
    /// Execution progress update
    Progress {
        request_id: String,
        percent: f32,
        message: Option<String>,
    },
    /// Function execution completed
    Complete {
        request_id: String,
        success: bool,
        result: Option<serde_json::Value>,
        error: Option<String>,
        duration_ms: u64,
    },
    /// Heartbeat to keep connection alive
    Heartbeat {
        timestamp: String,
    },
}

impl StreamEvent {
    /// Convert to SSE event
    fn to_sse_event(&self) -> Result<Event, Infallible> {
        let event_type = match self {
            StreamEvent::Start { .. } => "start",
            StreamEvent::Log { .. } => "log",
            StreamEvent::Output { .. } => "output",
            StreamEvent::Progress { .. } => "progress",
            StreamEvent::Complete { .. } => "complete",
            StreamEvent::Heartbeat { .. } => "heartbeat",
        };

        let data = serde_json::to_string(self).unwrap_or_else(|_| "{}".to_string());

        Ok(Event::default().event(event_type).data(data))
    }
}

/// Stream request parameters
#[derive(Debug, Deserialize)]
pub struct StreamRequest {
    /// Input payload for the function
    pub payload: Option<serde_json::Value>,
    /// Timeout in milliseconds
    pub timeout_ms: Option<u64>,
    /// Whether to include logs in stream
    #[serde(default = "default_include_logs")]
    pub include_logs: bool,
}

fn default_include_logs() -> bool {
    true
}

/// Create an SSE stream for function execution
pub async fn stream_function(
    Path(function_name): Path<String>,
    State(_server): State<Arc<ApiServer>>,
    Json(request): Json<StreamRequest>,
) -> Sse<impl Stream<Item = Result<Event, Infallible>>> {
    let request_id = uuid::Uuid::new_v4().to_string();
    let _timeout_ms = request.timeout_ms.unwrap_or(30_000);

    info!(
        function = %function_name,
        request_id = %request_id,
        "Starting SSE stream for function"
    );

    // Create channel for streaming events
    let (tx, rx) = mpsc::channel::<StreamEvent>(100);
    let stream = ReceiverStream::new(rx);

    // Spawn task to execute function and stream results
    let function_name_clone = function_name.clone();
    let payload = request.payload.unwrap_or(serde_json::Value::Null);
    let include_logs = request.include_logs;

    tokio::spawn(async move {
        // Send start event
        let _ = tx
            .send(StreamEvent::Start {
                request_id: request_id.clone(),
                function_name: function_name_clone.clone(),
                timestamp: chrono::Utc::now().to_rfc3339(),
            })
            .await;

        // Execute function (this would integrate with actual runtime)
        let start_time = std::time::Instant::now();

        // Simulated execution with progress updates
        // In production, this would hook into the actual runtime's output stream
        for i in 0..5 {
            tokio::time::sleep(Duration::from_millis(200)).await;

            // Send progress
            let _ = tx
                .send(StreamEvent::Progress {
                    request_id: request_id.clone(),
                    percent: (i + 1) as f32 * 20.0,
                    message: Some(format!("Processing step {}", i + 1)),
                })
                .await;

            // Send log if enabled
            if include_logs {
                let _ = tx
                    .send(StreamEvent::Log {
                        request_id: request_id.clone(),
                        level: "info".to_string(),
                        message: format!("Processing step {} completed", i + 1),
                        timestamp: chrono::Utc::now().to_rfc3339(),
                    })
                    .await;
            }
        }

        let duration_ms = start_time.elapsed().as_millis() as u64;

        // Send completion event
        let _ = tx
            .send(StreamEvent::Complete {
                request_id: request_id.clone(),
                success: true,
                result: Some(serde_json::json!({
                    "processed": true,
                    "input": payload,
                    "message": "Function executed successfully"
                })),
                error: None,
                duration_ms,
            })
            .await;

        debug!(
            request_id = %request_id,
            duration_ms = duration_ms,
            "SSE stream completed"
        );
    });

    // Convert stream events to SSE events
    let sse_stream = stream.map(|event: StreamEvent| event.to_sse_event());

    Sse::new(sse_stream).keep_alive(
        KeepAlive::new()
            .interval(Duration::from_secs(15))
            .text("heartbeat"),
    )
}

/// Stream multiple functions in parallel (fan-out)
pub async fn stream_fanout(
    State(_server): State<Arc<ApiServer>>,
    Json(request): Json<FanoutRequest>,
) -> Sse<impl Stream<Item = Result<Event, Infallible>>> {
    let request_id = uuid::Uuid::new_v4().to_string();

    info!(
        request_id = %request_id,
        functions = ?request.functions,
        "Starting fan-out SSE stream"
    );

    let (tx, rx) = mpsc::channel::<StreamEvent>(100);
    let stream = ReceiverStream::new(rx);

    // Spawn tasks for each function
    for (idx, func_name) in request.functions.iter().enumerate() {
        let tx_clone = tx.clone();
        let func_name = func_name.clone();
        let request_id = request_id.clone();
        let _payload = request.payload.clone().unwrap_or(serde_json::Value::Null);

        tokio::spawn(async move {
            // Simulated execution for each function
            let start = std::time::Instant::now();

            // Send start
            let _ = tx_clone
                .send(StreamEvent::Start {
                    request_id: request_id.clone(),
                    function_name: func_name.clone(),
                    timestamp: chrono::Utc::now().to_rfc3339(),
                })
                .await;

            // Simulated delay based on function index
            tokio::time::sleep(Duration::from_millis(100 * (idx as u64 + 1))).await;

            // Send result
            let _ = tx_clone
                .send(StreamEvent::Complete {
                    request_id: request_id.clone(),
                    success: true,
                    result: Some(serde_json::json!({
                        "function": func_name,
                        "index": idx,
                        "result": "ok"
                    })),
                    error: None,
                    duration_ms: start.elapsed().as_millis() as u64,
                })
                .await;
        });
    }

    let sse_stream = stream.map(|event: StreamEvent| event.to_sse_event());

    Sse::new(sse_stream).keep_alive(
        KeepAlive::new()
            .interval(Duration::from_secs(15))
            .text("heartbeat"),
    )
}

/// Fan-out request for multiple functions
#[derive(Debug, Deserialize)]
pub struct FanoutRequest {
    /// List of function names to execute
    pub functions: Vec<String>,
    /// Shared payload for all functions
    pub payload: Option<serde_json::Value>,
    /// Timeout per function in milliseconds
    pub timeout_ms: Option<u64>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_stream_event_serialization() {
        let event = StreamEvent::Start {
            request_id: "test-123".to_string(),
            function_name: "my-func".to_string(),
            timestamp: "2024-01-01T00:00:00Z".to_string(),
        };

        let json = serde_json::to_string(&event).expect("Should serialize");
        assert!(json.contains("\"type\":\"start\""));
        assert!(json.contains("\"request_id\":\"test-123\""));
    }

    #[test]
    fn test_complete_event() {
        let event = StreamEvent::Complete {
            request_id: "test-456".to_string(),
            success: true,
            result: Some(serde_json::json!({"data": 42})),
            error: None,
            duration_ms: 150,
        };

        let json = serde_json::to_string(&event).expect("Should serialize");
        assert!(json.contains("\"type\":\"complete\""));
        assert!(json.contains("\"success\":true"));
        assert!(json.contains("\"duration_ms\":150"));
    }

    #[test]
    fn test_progress_event() {
        let event = StreamEvent::Progress {
            request_id: "test-789".to_string(),
            percent: 50.0,
            message: Some("Halfway done".to_string()),
        };

        let json = serde_json::to_string(&event).expect("Should serialize");
        assert!(json.contains("\"type\":\"progress\""));
        assert!(json.contains("\"percent\":50"));
    }

    #[test]
    fn test_log_event() {
        let event = StreamEvent::Log {
            request_id: "test-log".to_string(),
            level: "warn".to_string(),
            message: "Something happened".to_string(),
            timestamp: "2024-01-01T12:00:00Z".to_string(),
        };

        let json = serde_json::to_string(&event).expect("Should serialize");
        assert!(json.contains("\"type\":\"log\""));
        assert!(json.contains("\"level\":\"warn\""));
    }

    #[test]
    fn test_fanout_request_deserialization() {
        let json = r#"{
            "functions": ["func-a", "func-b", "func-c"],
            "payload": {"key": "value"},
            "timeout_ms": 5000
        }"#;

        let request: FanoutRequest = serde_json::from_str(json).expect("Should deserialize");
        assert_eq!(request.functions.len(), 3);
        assert_eq!(request.timeout_ms, Some(5000));
    }
}
