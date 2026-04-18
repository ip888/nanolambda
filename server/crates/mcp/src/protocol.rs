//! JSON-RPC 2.0 + MCP protocol types.
//!
//! We deliberately hand-roll the minimal subset we need instead of pulling an
//! extra SDK dependency. Every field we serialize is shaped to match the MCP
//! 2025-06-18 wire format so off-the-shelf clients (Claude Desktop, Cursor,
//! Zed, mcp-inspector) interoperate without adapters.

use serde::{Deserialize, Serialize};
use serde_json::Value;

pub const JSONRPC_VERSION: &str = "2.0";
pub const MCP_PROTOCOL_VERSION: &str = "2025-06-18";

/// Reserved JSON-RPC error codes plus MCP-specific extensions.
#[derive(Debug, Clone, Copy)]
pub enum ErrorCode {
    ParseError,
    InvalidRequest,
    MethodNotFound,
    InvalidParams,
    InternalError,
    ToolExecutionFailed,
}

impl ErrorCode {
    pub const fn code(self) -> i32 {
        match self {
            Self::ParseError => -32_700,
            Self::InvalidRequest => -32_600,
            Self::MethodNotFound => -32_601,
            Self::InvalidParams => -32_602,
            Self::InternalError => -32_603,
            Self::ToolExecutionFailed => -32_000,
        }
    }
}

/// Errors surfaced back to the MCP client. `thiserror` gives us clean
/// propagation while `into_rpc_error` adapts to the wire shape.
#[derive(Debug, thiserror::Error)]
pub enum McpError {
    #[error("invalid params: {0}")]
    InvalidParams(String),
    #[error("method not found: {0}")]
    MethodNotFound(String),
    #[error("tool execution failed: {0}")]
    ToolFailed(String),
    #[error(transparent)]
    Other(#[from] anyhow::Error),
}

impl McpError {
    pub fn code(&self) -> ErrorCode {
        match self {
            Self::InvalidParams(_) => ErrorCode::InvalidParams,
            Self::MethodNotFound(_) => ErrorCode::MethodNotFound,
            Self::ToolFailed(_) => ErrorCode::ToolExecutionFailed,
            Self::Other(_) => ErrorCode::InternalError,
        }
    }
}

/// Incoming JSON-RPC request. `id` is absent on notifications; we currently
/// ignore notifications (the spec permits servers to do so).
#[derive(Debug, Deserialize)]
pub struct JsonRpcRequest {
    pub jsonrpc: String,
    #[serde(default)]
    pub id: Option<Value>,
    pub method: String,
    #[serde(default)]
    pub params: Option<Value>,
}

/// Outgoing JSON-RPC response. Exactly one of `result` / `error` is set.
#[derive(Debug, Serialize)]
pub struct JsonRpcResponse {
    pub jsonrpc: &'static str,
    pub id: Value,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<RpcErrorPayload>,
}

#[derive(Debug, Serialize)]
pub struct RpcErrorPayload {
    pub code: i32,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<Value>,
}

impl JsonRpcResponse {
    pub fn ok(id: Value, result: Value) -> Self {
        Self {
            jsonrpc: JSONRPC_VERSION,
            id,
            result: Some(result),
            error: None,
        }
    }

    pub fn err(id: Value, err: &McpError) -> Self {
        Self {
            jsonrpc: JSONRPC_VERSION,
            id,
            result: None,
            error: Some(RpcErrorPayload {
                code: err.code().code(),
                message: err.to_string(),
                data: None,
            }),
        }
    }
}

/// `initialize` response payload. Advertises capabilities and server identity.
#[derive(Debug, Serialize)]
pub struct InitializeResult {
    #[serde(rename = "protocolVersion")]
    pub protocol_version: &'static str,
    pub capabilities: ServerCapabilities,
    #[serde(rename = "serverInfo")]
    pub server_info: ServerInfo,
}

#[derive(Debug, Serialize)]
pub struct ServerCapabilities {
    pub tools: ToolsCapability,
}

#[derive(Debug, Serialize)]
pub struct ToolsCapability {
    #[serde(rename = "listChanged")]
    pub list_changed: bool,
}

#[derive(Debug, Serialize)]
pub struct ServerInfo {
    pub name: &'static str,
    pub version: &'static str,
}

/// Tool descriptor returned by `tools/list`.
#[derive(Debug, Serialize)]
pub struct ToolDescriptor {
    pub name: &'static str,
    pub description: &'static str,
    #[serde(rename = "inputSchema")]
    pub input_schema: Value,
}

/// Tool call payload (`tools/call`).
#[derive(Debug, Deserialize)]
pub struct ToolCallRequest {
    pub name: String,
    #[serde(default)]
    pub arguments: Value,
}

/// Tool call response. `content` is an array of typed blocks; MCP clients
/// render them in order. We only use text blocks here.
#[derive(Debug, Serialize)]
pub struct ToolCallResult {
    pub content: Vec<ContentBlock>,
    #[serde(rename = "isError", skip_serializing_if = "is_false")]
    pub is_error: bool,
    #[serde(rename = "_meta", skip_serializing_if = "Option::is_none")]
    pub meta: Option<Value>,
}

#[derive(Debug, Serialize)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum ContentBlock {
    Text { text: String },
}

const fn is_false(b: &bool) -> bool {
    !*b
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn error_codes_match_jsonrpc_spec() {
        assert_eq!(ErrorCode::ParseError.code(), -32_700);
        assert_eq!(ErrorCode::InvalidRequest.code(), -32_600);
        assert_eq!(ErrorCode::MethodNotFound.code(), -32_601);
        assert_eq!(ErrorCode::InvalidParams.code(), -32_602);
        assert_eq!(ErrorCode::InternalError.code(), -32_603);
    }

    #[test]
    fn response_serializes_without_null_error_field() {
        let resp = JsonRpcResponse::ok(Value::from(1), Value::from("ok"));
        let json = serde_json::to_string(&resp).unwrap();
        assert!(!json.contains("\"error\""));
        assert!(json.contains("\"result\":\"ok\""));
    }

    #[test]
    fn error_response_excludes_result_field() {
        let err = McpError::InvalidParams("missing 'code'".into());
        let resp = JsonRpcResponse::err(Value::from(2), &err);
        let json = serde_json::to_string(&resp).unwrap();
        assert!(!json.contains("\"result\""));
        assert!(json.contains("\"code\":-32602"));
    }
}
