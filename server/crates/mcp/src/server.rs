//! Stdio-driven MCP request loop.
//!
//! Reads newline-delimited JSON-RPC 2.0 messages from stdin, dispatches to
//! the appropriate handler, and writes responses to stdout. stderr is
//! reserved for log output so it never clashes with the wire protocol.

use serde_json::{Value, json};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};

use crate::protocol::{
    ContentBlock, InitializeResult, JsonRpcRequest, JsonRpcResponse, MCP_PROTOCOL_VERSION,
    McpError, ServerCapabilities, ServerInfo, ToolCallRequest, ToolCallResult, ToolsCapability,
};
use crate::sandbox::{HttpSandboxClient, SandboxClient, SandboxRequest};
use crate::tools;

/// Runtime configuration for the MCP server.
#[derive(Debug, Clone)]
pub struct Config {
    pub base_url: String,
    pub api_key: Option<String>,
    pub timeout_ms: u64,
    pub memory_mb: u32,
}

/// The MCP server. Generic over the sandbox client so tests can drop in a fake.
pub struct Server<C: SandboxClient = HttpSandboxClient> {
    config: Config,
    client: C,
}

impl Server<HttpSandboxClient> {
    pub fn new(config: Config) -> Self {
        let client = HttpSandboxClient::new(config.base_url.clone(), config.api_key.clone())
            .expect("reqwest client builder should never fail with default settings");
        Self { config, client }
    }
}

impl<C: SandboxClient> Server<C> {
    pub fn with_client(config: Config, client: C) -> Self {
        Self { config, client }
    }

    /// Run the stdio loop until EOF.
    pub async fn run(self) -> anyhow::Result<()> {
        let stdin = tokio::io::stdin();
        let mut reader = BufReader::new(stdin);
        let mut stdout = tokio::io::stdout();
        let mut line = String::new();

        loop {
            line.clear();
            let n = reader.read_line(&mut line).await?;
            if n == 0 {
                tracing::info!("stdin closed; shutting down");
                return Ok(());
            }

            let trimmed = line.trim();
            if trimmed.is_empty() {
                continue;
            }

            let response = self.handle_raw(trimmed).await;
            if let Some(resp) = response {
                let mut bytes = serde_json::to_vec(&resp)?;
                bytes.push(b'\n');
                stdout.write_all(&bytes).await?;
                stdout.flush().await?;
            }
        }
    }

    /// Parse a single line and produce the response. Returns `None` for
    /// notifications (requests without an `id`) per JSON-RPC 2.0.
    async fn handle_raw(&self, raw: &str) -> Option<JsonRpcResponse> {
        let req: JsonRpcRequest = match serde_json::from_str(raw) {
            Ok(r) => r,
            Err(e) => {
                tracing::warn!(error = %e, "failed to parse JSON-RPC message");
                return Some(JsonRpcResponse::err(
                    Value::Null,
                    &McpError::InvalidParams(format!("parse error: {e}")),
                ));
            }
        };

        let id = req.id.clone()?; // notification → no response

        let result = self.dispatch(&req.method, req.params.unwrap_or(Value::Null)).await;
        match result {
            Ok(value) => Some(JsonRpcResponse::ok(id, value)),
            Err(err) => {
                tracing::warn!(method = %req.method, error = %err, "rpc error");
                Some(JsonRpcResponse::err(id, &err))
            }
        }
    }

    async fn dispatch(&self, method: &str, params: Value) -> Result<Value, McpError> {
        match method {
            "initialize" => Ok(serde_json::to_value(InitializeResult {
                protocol_version: MCP_PROTOCOL_VERSION,
                capabilities: ServerCapabilities {
                    tools: ToolsCapability { list_changed: false },
                },
                server_info: ServerInfo {
                    name: "nanolambda-mcp",
                    version: env!("CARGO_PKG_VERSION"),
                },
            })
            .map_err(|e| McpError::Other(e.into()))?),

            "notifications/initialized" | "shutdown" | "exit" => Ok(Value::Null),

            "ping" => Ok(json!({})),

            "tools/list" => Ok(json!({ "tools": tools::descriptors() })),

            "tools/call" => {
                let call: ToolCallRequest = serde_json::from_value(params)
                    .map_err(|e| McpError::InvalidParams(e.to_string()))?;
                let result = self.call_tool(&call.name, &call.arguments).await?;
                serde_json::to_value(result).map_err(|e| McpError::Other(e.into()))
            }

            other => Err(McpError::MethodNotFound(other.to_string())),
        }
    }

    async fn call_tool(&self, name: &str, arguments: &Value) -> Result<ToolCallResult, McpError> {
        // Validate the tool exists before spending an HTTP round-trip.
        if !tools::descriptors().iter().any(|d| d.name == name) {
            return Err(McpError::MethodNotFound(format!("unknown tool: {name}")));
        }

        let timeout_ms = arguments
            .get("timeout_ms")
            .and_then(Value::as_u64)
            .unwrap_or(self.config.timeout_ms);

        let outcome = self
            .client
            .invoke(SandboxRequest {
                tool: name,
                args: arguments,
                timeout_ms,
                memory_mb: self.config.memory_mb,
            })
            .await;

        match outcome {
            Ok(result) => {
                let is_error = result.exit_code != 0;
                let meta = json!({
                    "exit_code": result.exit_code,
                    "duration_ms": result.duration_ms,
                    "cold_start": result.cold_start,
                });
                Ok(ToolCallResult {
                    content: vec![ContentBlock::Text {
                        text: result.into_display(),
                    }],
                    is_error,
                    meta: Some(meta),
                })
            }
            Err(McpError::ToolFailed(msg)) => Ok(ToolCallResult {
                content: vec![ContentBlock::Text { text: msg }],
                is_error: true,
                meta: None,
            }),
            Err(other) => Err(other),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sandbox::SandboxResult;

    struct FakeClient;

    impl SandboxClient for FakeClient {
        async fn invoke(
            &self,
            req: SandboxRequest<'_>,
        ) -> Result<SandboxResult, McpError> {
            Ok(SandboxResult {
                stdout: format!("ran {}", req.tool),
                stderr: String::new(),
                exit_code: 0,
                duration_ms: 12,
                cold_start: true,
            })
        }
    }

    fn server() -> Server<FakeClient> {
        Server::with_client(
            Config {
                base_url: "http://localhost:0".into(),
                api_key: None,
                timeout_ms: 5000,
                memory_mb: 128,
            },
            FakeClient,
        )
    }

    #[tokio::test]
    async fn initialize_advertises_tools_capability() {
        let s = server();
        let raw = r#"{"jsonrpc":"2.0","id":1,"method":"initialize","params":{}}"#;
        let resp = s.handle_raw(raw).await.unwrap();
        let body = serde_json::to_value(&resp).unwrap();
        assert_eq!(body["jsonrpc"], "2.0");
        assert_eq!(body["result"]["serverInfo"]["name"], "nanolambda-mcp");
        assert!(body["result"]["capabilities"]["tools"].is_object());
    }

    #[tokio::test]
    async fn tools_list_returns_nonempty_catalog() {
        let s = server();
        let raw = r#"{"jsonrpc":"2.0","id":2,"method":"tools/list"}"#;
        let resp = s.handle_raw(raw).await.unwrap();
        let body = serde_json::to_value(&resp).unwrap();
        let tools = body["result"]["tools"].as_array().unwrap();
        assert!(tools.len() >= 5);
    }

    #[tokio::test]
    async fn tools_call_forwards_to_client_and_returns_text_content() {
        let s = server();
        let raw = r#"{"jsonrpc":"2.0","id":3,"method":"tools/call","params":{"name":"execute_python","arguments":{"code":"print(1)"}}}"#;
        let resp = s.handle_raw(raw).await.unwrap();
        let body = serde_json::to_value(&resp).unwrap();
        assert_eq!(body["result"]["content"][0]["text"], "ran execute_python");
        assert_eq!(body["result"]["isError"], Value::Null); // skipped because false
        assert_eq!(body["result"]["_meta"]["cold_start"], true);
    }

    #[tokio::test]
    async fn unknown_tool_returns_method_not_found() {
        let s = server();
        let raw = r#"{"jsonrpc":"2.0","id":4,"method":"tools/call","params":{"name":"nope","arguments":{}}}"#;
        let resp = s.handle_raw(raw).await.unwrap();
        let body = serde_json::to_value(&resp).unwrap();
        assert_eq!(body["error"]["code"], -32601);
    }

    #[tokio::test]
    async fn notification_without_id_returns_no_response() {
        let s = server();
        let raw = r#"{"jsonrpc":"2.0","method":"notifications/initialized"}"#;
        assert!(s.handle_raw(raw).await.is_none());
    }

    #[tokio::test]
    async fn unknown_method_returns_method_not_found() {
        let s = server();
        let raw = r#"{"jsonrpc":"2.0","id":5,"method":"bogus"}"#;
        let resp = s.handle_raw(raw).await.unwrap();
        let body = serde_json::to_value(&resp).unwrap();
        assert_eq!(body["error"]["code"], -32601);
    }
}
