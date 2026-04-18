//! HTTP client that forwards tool calls to the NanoLambda control plane.
//!
//! Kept behind a trait so tests can drive the server with an in-memory fake
//! without spinning up a real HTTP backend.

use std::time::Duration;

use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

use crate::protocol::McpError;

/// Tool invocation payload. `tool` is the MCP tool name; `args` is forwarded
/// verbatim as the invoke request body.
#[derive(Debug, Serialize)]
pub struct SandboxRequest<'a> {
    pub tool: &'a str,
    pub args: &'a Value,
    pub timeout_ms: u64,
    pub memory_mb: u32,
}

/// Normalized result shape returned by every sandbox backend.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct SandboxResult {
    #[serde(default)]
    pub stdout: String,
    #[serde(default)]
    pub stderr: String,
    #[serde(default)]
    pub exit_code: i32,
    #[serde(default)]
    pub duration_ms: u64,
    #[serde(default)]
    pub cold_start: bool,
}

impl SandboxResult {
    /// Render the result as an MCP text block. Stdout is primary, stderr is
    /// appended if non-empty so agents see failure diagnostics inline.
    pub fn into_display(self) -> String {
        if self.stderr.is_empty() {
            self.stdout
        } else if self.stdout.is_empty() {
            format!("[stderr]\n{}", self.stderr)
        } else {
            format!("{}\n[stderr]\n{}", self.stdout, self.stderr)
        }
    }
}

/// Abstract backend so we can unit-test the dispatch loop without HTTP.
/// Native `async fn in trait` (Rust 2024) — no `async-trait` dependency needed.
#[allow(async_fn_in_trait)]
pub trait SandboxClient: Send + Sync {
    async fn invoke(&self, req: SandboxRequest<'_>) -> Result<SandboxResult, McpError>;
}

/// Default HTTP implementation talking to a NanoLambda control plane.
#[derive(Debug, Clone)]
pub struct HttpSandboxClient {
    client: reqwest::Client,
    base_url: String,
    api_key: Option<String>,
}

impl HttpSandboxClient {
    pub fn new(base_url: String, api_key: Option<String>) -> anyhow::Result<Self> {
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(120))
            .user_agent(concat!("nanolambda-mcp/", env!("CARGO_PKG_VERSION")))
            .build()?;
        Ok(Self {
            client,
            base_url,
            api_key,
        })
    }
}

impl SandboxClient for HttpSandboxClient {
    async fn invoke(&self, req: SandboxRequest<'_>) -> Result<SandboxResult, McpError> {
        let body = json!({
            "tool": req.tool,
            "args": req.args,
            "timeout_ms": req.timeout_ms,
            "memory_mb": req.memory_mb,
        });

        let url = format!("{}/sandbox/invoke", self.base_url.trim_end_matches('/'));
        let mut builder = self.client.post(&url).json(&body);
        if let Some(key) = &self.api_key {
            builder = builder.header("authorization", format!("Bearer {key}"));
        }

        let resp = builder
            .send()
            .await
            .map_err(|e| McpError::ToolFailed(format!("transport: {e}")))?;

        let status = resp.status();
        if !status.is_success() {
            let text = resp.text().await.unwrap_or_default();
            return Err(McpError::ToolFailed(format!(
                "control plane returned {status}: {text}"
            )));
        }

        resp.json::<SandboxResult>()
            .await
            .map_err(|e| McpError::ToolFailed(format!("malformed response: {e}")))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn display_combines_stdout_and_stderr() {
        let r = SandboxResult {
            stdout: "hi".into(),
            stderr: "warn".into(),
            ..Default::default()
        };
        let s = r.into_display();
        assert!(s.contains("hi"));
        assert!(s.contains("[stderr]"));
        assert!(s.contains("warn"));
    }

    #[test]
    fn display_is_just_stdout_when_stderr_empty() {
        let r = SandboxResult {
            stdout: "ok".into(),
            ..Default::default()
        };
        assert_eq!(r.into_display(), "ok");
    }

    impl Default for SandboxResult {
        fn default() -> Self {
            Self {
                stdout: String::new(),
                stderr: String::new(),
                exit_code: 0,
                duration_ms: 0,
                cold_start: false,
            }
        }
    }
}
