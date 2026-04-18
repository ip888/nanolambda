//! E2B platform adapter for reproducible head-to-head benchmarks.
//!
//! Uses the E2B sandbox HTTP API. Credentials are loaded from `E2B_API_KEY`;
//! when the variable is absent the adapter refuses to run so CI runs against
//! NanoLambda only remain deterministic.
//!
//! API docs: <https://e2b.dev/docs/api>.

use std::time::Duration;

use serde::{Deserialize, Serialize};
use serde_json::json;

use crate::platforms::Platform;
use crate::workloads::Workload;

const E2B_DEFAULT_HOST: &str = "https://api.e2b.dev";
const E2B_DEFAULT_TEMPLATE: &str = "base";

#[derive(Debug, Deserialize)]
struct CreateSandboxResponse {
    #[serde(alias = "sandboxID", alias = "id")]
    sandbox_id: String,
}

#[derive(Debug, Deserialize)]
struct ExecResponse {
    #[serde(default)]
    stdout: String,
    #[serde(default)]
    stderr: String,
    #[serde(default, alias = "exitCode", alias = "exit_code")]
    exit_code: i32,
}

#[derive(Debug, Serialize)]
struct ExecBody<'a> {
    cmd: &'a str,
}

pub struct E2B {
    client: reqwest::Client,
    host: String,
    api_key: String,
    template: String,
    /// Active sandbox id, swapped out every `ensure_cold_state`.
    active: tokio::sync::Mutex<Option<String>>,
}

impl E2B {
    /// Build an E2B client. Errors if `E2B_API_KEY` is unset so misconfigured
    /// runs fail loudly instead of silently benchmarking 404s.
    pub fn new() -> anyhow::Result<Self> {
        let api_key = std::env::var("E2B_API_KEY").map_err(|_| {
            anyhow::anyhow!(
                "E2B_API_KEY not set — export it to enable the E2B benchmark platform"
            )
        })?;
        let host = std::env::var("E2B_HOST").unwrap_or_else(|_| E2B_DEFAULT_HOST.to_string());
        let template =
            std::env::var("E2B_TEMPLATE").unwrap_or_else(|_| E2B_DEFAULT_TEMPLATE.to_string());

        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(60))
            .user_agent("nanolambda-benchmarks/0.1")
            .build()?;

        Ok(Self {
            client,
            host,
            api_key,
            template,
            active: tokio::sync::Mutex::new(None),
        })
    }

    async fn create_sandbox(&self) -> anyhow::Result<String> {
        let url = format!("{}/sandboxes", self.host.trim_end_matches('/'));
        let resp = self
            .client
            .post(url)
            .header("X-API-Key", &self.api_key)
            .json(&json!({ "templateID": self.template }))
            .send()
            .await?;

        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            anyhow::bail!("E2B sandbox create failed ({status}): {body}");
        }

        let parsed: CreateSandboxResponse = resp.json().await?;
        Ok(parsed.sandbox_id)
    }

    async fn exec(&self, sandbox_id: &str, command: &str) -> anyhow::Result<ExecResponse> {
        let url = format!(
            "{}/sandboxes/{}/processes",
            self.host.trim_end_matches('/'),
            sandbox_id
        );
        let resp = self
            .client
            .post(url)
            .header("X-API-Key", &self.api_key)
            .json(&ExecBody { cmd: command })
            .send()
            .await?;

        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            anyhow::bail!("E2B exec failed ({status}): {body}");
        }

        Ok(resp.json().await?)
    }

    async fn kill(&self, sandbox_id: &str) -> anyhow::Result<()> {
        let url = format!(
            "{}/sandboxes/{}",
            self.host.trim_end_matches('/'),
            sandbox_id
        );
        let _ = self
            .client
            .delete(url)
            .header("X-API-Key", &self.api_key)
            .send()
            .await;
        Ok(())
    }

    async fn active_id(&self) -> anyhow::Result<String> {
        let mut guard = self.active.lock().await;
        if let Some(id) = guard.as_ref() {
            return Ok(id.clone());
        }
        let id = self.create_sandbox().await?;
        *guard = Some(id.clone());
        Ok(id)
    }
}

impl Platform for E2B {
    fn name(&self) -> &'static str {
        "E2B"
    }

    async fn deploy(&self, _workload: &Workload) -> anyhow::Result<()> {
        // E2B sandboxes are ephemeral; nothing to pre-deploy.
        Ok(())
    }

    async fn invoke(&self, workload: &Workload) -> anyhow::Result<serde_json::Value> {
        let id = self.active_id().await?;
        // Drive Python workloads through `python3 -c`; mirrors how agents use
        // E2B in practice (one-shot exec instead of long-running REPL).
        let code = workload.code().replace('"', r#"\""#);
        let payload = serde_json::to_string(&workload.payload())?;
        let payload_escaped = payload.replace('\'', r"'\''");
        let command = format!(
            "python3 -c \"{code}\nimport json,sys; print(json.dumps(handler(json.loads('{payload_escaped}'))))\""
        );
        let result = self.exec(&id, &command).await?;
        Ok(json!({
            "stdout": result.stdout,
            "stderr": result.stderr,
            "exit_code": result.exit_code,
        }))
    }

    async fn get_memory_usage(&self, _workload: &Workload) -> anyhow::Result<u64> {
        // E2B default template is 512 MB as of 2026-04.
        Ok(512)
    }

    async fn ensure_cold_state(&self, _workload: &Workload) -> anyhow::Result<()> {
        let mut guard = self.active.lock().await;
        if let Some(id) = guard.take() {
            self.kill(&id).await?;
        }
        Ok(())
    }

    async fn cleanup(&self, _workload: &Workload) -> anyhow::Result<()> {
        let mut guard = self.active.lock().await;
        if let Some(id) = guard.take() {
            self.kill(&id).await?;
        }
        Ok(())
    }
}
