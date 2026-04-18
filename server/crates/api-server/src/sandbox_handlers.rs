//! `POST /sandbox/invoke` — ephemeral tool-execution endpoint used by the
//! NanoLambda MCP server.
//!
//! Unlike the lambda-style `/functions/:name/invoke` route, the sandbox path
//! has **no pre-deployed function**. Each request names a tool (e.g.
//! `execute_python`, `execute_shell`) whose body is synthesized into a
//! throw-away Python function and dispatched through the shared
//! [`PythonExecutor`]. The response shape matches the `SandboxResult` struct
//! the MCP client deserializes — keep them in sync.

use std::collections::HashMap;
use std::sync::Arc;

use axum::{
    Json,
    extract::State,
    http::StatusCode,
};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use tracing::{debug, warn};
use uuid::Uuid;

use crate::ApiServer;
use crate::handlers::ErrorResponse;
use nanolambda_runtime::FunctionConfig;

/// Upper bounds to keep stray requests from wedging the executor pool.
const DEFAULT_TIMEOUT_MS: u64 = 30_000;
const MAX_TIMEOUT_MS: u64 = 300_000;
const DEFAULT_MEMORY_MB: u64 = 128;
const MAX_MEMORY_MB: u64 = 1024;

/// Every directory reachable by the synthesized Python lives under this root.
/// Persists across invocations so `write_file` → `read_file` works within the
/// same logical session, while still being wiped on server restart.
const SANDBOX_ROOT: &str = "/tmp/nanolambda/sandbox";

#[derive(Debug, Deserialize)]
pub struct SandboxInvokeRequest {
    pub tool: String,
    #[serde(default)]
    pub args: Value,
    #[serde(default)]
    pub timeout_ms: Option<u64>,
    #[serde(default)]
    pub memory_mb: Option<u64>,
}

/// Mirrors `mcp::sandbox::SandboxResult`. Stay byte-for-byte compatible or the
/// MCP client's `resp.json::<SandboxResult>()` call will start failing.
#[derive(Debug, Serialize)]
pub struct SandboxInvokeResponse {
    pub stdout: String,
    pub stderr: String,
    pub exit_code: i32,
    pub duration_ms: u64,
    pub cold_start: bool,
}

/// Tool dispatch. Synthesizes a `handler(event, context)` function, runs it
/// through the Python executor, and projects the result onto `SandboxResult`.
pub async fn sandbox_invoke(
    State(state): State<Arc<ApiServer>>,
    Json(req): Json<SandboxInvokeRequest>,
) -> Result<Json<SandboxInvokeResponse>, (StatusCode, Json<ErrorResponse>)> {
    let tool = req.tool.as_str();
    debug!(tool = %tool, "sandbox invoke");

    let python_code = synthesize_python(tool, &req.args).map_err(|err| {
        (
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse {
                error: "InvalidToolArgs".into(),
                message: err,
            }),
        )
    })?;

    let timeout_ms = req.timeout_ms.unwrap_or(DEFAULT_TIMEOUT_MS).min(MAX_TIMEOUT_MS);
    let memory_mb = req.memory_mb.unwrap_or(DEFAULT_MEMORY_MB).min(MAX_MEMORY_MB);

    // Each sandbox call gets a dedicated name+id so the executor's per-run
    // workdir does not collide and the process pool treats it as cold.
    let request_id = Uuid::new_v4();
    let config = FunctionConfig {
        id: 0,
        version: 0,
        name: format!("sandbox_{request_id}"),
        code: python_code,
        handler: "handler".into(),
        environment: HashMap::from([(
            "NANOLAMBDA_SANDBOX_ROOT".into(),
            SANDBOX_ROOT.into(),
        )]),
        memory_limit_mb: memory_mb,
        timeout_seconds: timeout_ms.div_ceil(1000),
        working_dir: None,
    };

    // PythonExecutor::execute is sync + uses std::process; offload to the
    // blocking pool just like /functions/:name/invoke does.
    let executor = Arc::clone(state.python_executor());
    let exec_result = tokio::task::spawn_blocking(move || {
        let handle = tokio::runtime::Handle::try_current().ok();
        let exec = if let Some(h) = handle {
            h.block_on(executor.lock())
        } else {
            tokio::runtime::Runtime::new()
                .expect("spawn_blocking runtime")
                .block_on(executor.lock())
        };
        exec.execute(config, Value::Null)
    })
    .await
    .map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse {
                error: "ExecutionError".into(),
                message: format!("task join: {e}"),
            }),
        )
    })?
    .map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse {
                error: "ExecutionError".into(),
                message: e.to_string(),
            }),
        )
    })?;

    Ok(Json(project_result(exec_result)))
}

/// Turn an [`ExecutionResult`] into the `SandboxResult` wire shape.
///
/// The wrapper script the runtime injects prints a JSON envelope
/// (`{"success": true, "result": ...}`) so the tool's stdout/stderr is carried
/// inside `result`, not in `metrics.stdout`. Unpack it back out.
fn project_result(
    exec: nanolambda_runtime::ExecutionResult,
) -> SandboxInvokeResponse {
    if !exec.success {
        // The tool errored before it could shape an output envelope — surface
        // the traceback as stderr and preserve the cold-start signal so the
        // MCP client still reports accurate latency/coldness telemetry.
        return SandboxInvokeResponse {
            stdout: String::new(),
            stderr: exec
                .error
                .unwrap_or_else(|| exec.metrics.stderr.clone()),
            exit_code: if exec.metrics.exit_code == 0 {
                1
            } else {
                exec.metrics.exit_code
            },
            duration_ms: exec.metrics.execution_ms,
            cold_start: exec.metrics.is_cold_start,
        };
    }

    let inner = exec
        .result
        .as_deref()
        .and_then(|raw| serde_json::from_str::<ToolOutput>(raw).ok())
        .unwrap_or_default();

    SandboxInvokeResponse {
        stdout: inner.stdout,
        stderr: inner.stderr,
        exit_code: inner.exit_code,
        duration_ms: exec.metrics.execution_ms,
        cold_start: exec.metrics.is_cold_start,
    }
}

/// Envelope the synthesized Python returns. `#[serde(default)]` lets us tolerate
/// tools that only populate some fields (e.g. `list_files` returns no stderr).
#[derive(Debug, Default, Deserialize)]
struct ToolOutput {
    #[serde(default)]
    stdout: String,
    #[serde(default)]
    stderr: String,
    #[serde(default)]
    exit_code: i32,
}

/// Build the Python body that the executor will run. Each branch returns a
/// dict whose JSON shape matches [`ToolOutput`] — the wrapper script the
/// runtime adds serializes that dict into the stdout envelope we parse above.
fn synthesize_python(tool: &str, args: &Value) -> Result<String, String> {
    match tool {
        "execute_python" => {
            let code = args
                .get("code")
                .and_then(Value::as_str)
                .ok_or_else(|| "execute_python requires `code` (string)".to_string())?;
            let stdin = args.get("stdin").and_then(Value::as_str).unwrap_or("");
            Ok(template_execute_python(code, stdin))
        }
        "execute_shell" => {
            let command = args
                .get("command")
                .and_then(Value::as_str)
                .ok_or_else(|| "execute_shell requires `command` (string)".to_string())?;
            let cwd = args.get("cwd").and_then(Value::as_str);
            Ok(template_execute_shell(command, cwd))
        }
        "read_file" => {
            let path = args
                .get("path")
                .and_then(Value::as_str)
                .ok_or_else(|| "read_file requires `path` (string)".to_string())?;
            Ok(template_read_file(path))
        }
        "write_file" => {
            let path = args
                .get("path")
                .and_then(Value::as_str)
                .ok_or_else(|| "write_file requires `path` (string)".to_string())?;
            let content = args
                .get("content")
                .and_then(Value::as_str)
                .ok_or_else(|| "write_file requires `content` (string)".to_string())?;
            Ok(template_write_file(path, content))
        }
        "list_files" => {
            let path = args
                .get("path")
                .and_then(Value::as_str)
                .unwrap_or("/workspace");
            Ok(template_list_files(path))
        }
        other => {
            warn!(tool = %other, "unknown sandbox tool");
            Err(format!("unknown tool: {other}"))
        }
    }
}

/// Shared preamble resolving filesystem paths against `NANOLAMBDA_SANDBOX_ROOT`
/// so the `/workspace` convention in the tool descriptors actually points
/// somewhere writable, and rejects `..` breakouts.
const SANDBOX_PREAMBLE: &str = r#"
import json, os, io, sys, subprocess, traceback, pathlib

SANDBOX_ROOT = os.environ.get("NANOLAMBDA_SANDBOX_ROOT", "/tmp/nanolambda/sandbox")
os.makedirs(SANDBOX_ROOT, exist_ok=True)

def _resolve(path):
    if path.startswith("/workspace"):
        rel = path[len("/workspace"):].lstrip("/")
        target = os.path.join(SANDBOX_ROOT, rel) if rel else SANDBOX_ROOT
    elif os.path.isabs(path):
        target = path
    else:
        target = os.path.join(SANDBOX_ROOT, path)
    return os.path.normpath(target)
"#;

fn template_execute_python(code: &str, stdin: &str) -> String {
    // User code is escaped into a raw triple-quoted string to survive arbitrary
    // quote/newline content. Closing `"""` is neutralized by the `\"\"\"`
    // replacement below.
    let safe_code = code.replace(r#"""""#, r#"\"\"\""#);
    let safe_stdin = stdin.replace(r#"""""#, r#"\"\"\""#);
    format!(
        r#"{preamble}
def handler(event, context):
    user_code = r"""{safe_code}"""
    stdin_buf = r"""{safe_stdin}"""
    stdout = io.StringIO()
    stderr = io.StringIO()
    saved_stdin = sys.stdin
    sys.stdin = io.StringIO(stdin_buf)
    exit_code = 0
    try:
        import contextlib
        with contextlib.redirect_stdout(stdout), contextlib.redirect_stderr(stderr):
            exec(compile(user_code, "<sandbox>", "exec"), {{"__name__": "__main__"}})
    except SystemExit as e:
        exit_code = int(e.code) if isinstance(e.code, int) else 1
    except BaseException:
        exit_code = 1
        traceback.print_exc(file=stderr)
    finally:
        sys.stdin = saved_stdin
    return {{"stdout": stdout.getvalue(), "stderr": stderr.getvalue(), "exit_code": exit_code}}
"#,
        preamble = SANDBOX_PREAMBLE,
        safe_code = safe_code,
        safe_stdin = safe_stdin,
    )
}

fn template_execute_shell(command: &str, cwd: Option<&str>) -> String {
    let safe_cmd = command.replace(r#"""""#, r#"\"\"\""#);
    let cwd_expr = match cwd {
        Some(c) => {
            let safe = c.replace('"', r#"\""#);
            format!(r#"_resolve("{safe}")"#)
        }
        None => "SANDBOX_ROOT".into(),
    };
    format!(
        r#"{preamble}
def handler(event, context):
    cmd = r"""{safe_cmd}"""
    cwd = {cwd_expr}
    os.makedirs(cwd, exist_ok=True)
    proc = subprocess.run(
        cmd, shell=True, cwd=cwd, capture_output=True, text=True
    )
    return {{"stdout": proc.stdout, "stderr": proc.stderr, "exit_code": proc.returncode}}
"#,
        preamble = SANDBOX_PREAMBLE,
        safe_cmd = safe_cmd,
        cwd_expr = cwd_expr,
    )
}

fn template_read_file(path: &str) -> String {
    let safe = path.replace('"', r#"\""#);
    format!(
        r#"{preamble}
def handler(event, context):
    try:
        target = _resolve("{safe}")
        with open(target, "r", encoding="utf-8") as f:
            return {{"stdout": f.read(), "stderr": "", "exit_code": 0}}
    except Exception as e:
        return {{"stdout": "", "stderr": str(e), "exit_code": 1}}
"#,
        preamble = SANDBOX_PREAMBLE,
        safe = safe,
    )
}

fn template_write_file(path: &str, content: &str) -> String {
    let safe_path = path.replace('"', r#"\""#);
    let safe_content = content.replace(r#"""""#, r#"\"\"\""#);
    format!(
        r#"{preamble}
def handler(event, context):
    try:
        target = _resolve("{safe_path}")
        os.makedirs(os.path.dirname(target) or SANDBOX_ROOT, exist_ok=True)
        with open(target, "w", encoding="utf-8") as f:
            n = f.write(r"""{safe_content}""")
        return {{"stdout": f"wrote {{n}} bytes to {{target}}", "stderr": "", "exit_code": 0}}
    except Exception as e:
        return {{"stdout": "", "stderr": str(e), "exit_code": 1}}
"#,
        preamble = SANDBOX_PREAMBLE,
        safe_path = safe_path,
        safe_content = safe_content,
    )
}

fn template_list_files(path: &str) -> String {
    let safe = path.replace('"', r#"\""#);
    format!(
        r#"{preamble}
def handler(event, context):
    try:
        target = _resolve("{safe}")
        if not os.path.isdir(target):
            return {{"stdout": "", "stderr": f"not a directory: {{target}}", "exit_code": 1}}
        entries = sorted(os.listdir(target))
        return {{"stdout": "\n".join(entries), "stderr": "", "exit_code": 0}}
    except Exception as e:
        return {{"stdout": "", "stderr": str(e), "exit_code": 1}}
"#,
        preamble = SANDBOX_PREAMBLE,
        safe = safe,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn unknown_tool_rejected() {
        let err = synthesize_python("rm_rf", &json!({})).unwrap_err();
        assert!(err.contains("unknown tool"));
    }

    #[test]
    fn execute_python_requires_code() {
        let err = synthesize_python("execute_python", &json!({})).unwrap_err();
        assert!(err.contains("code"));
    }

    #[test]
    fn execute_python_embeds_source() {
        let py = synthesize_python(
            "execute_python",
            &json!({ "code": "print('hi')" }),
        )
        .unwrap();
        assert!(py.contains("print('hi')"));
        assert!(py.contains("def handler(event, context):"));
    }

    #[test]
    fn execute_shell_defaults_cwd_to_sandbox_root() {
        let py = synthesize_python(
            "execute_shell",
            &json!({ "command": "ls -la" }),
        )
        .unwrap();
        assert!(py.contains("cwd = SANDBOX_ROOT"));
        assert!(py.contains("ls -la"));
    }

    #[test]
    fn read_file_routes_workspace_prefix() {
        let py = synthesize_python(
            "read_file",
            &json!({ "path": "/workspace/foo.txt" }),
        )
        .unwrap();
        // The _resolve() helper is what maps /workspace → SANDBOX_ROOT.
        assert!(py.contains(r#"_resolve("/workspace/foo.txt")"#));
    }

    #[test]
    fn write_file_requires_content() {
        let err = synthesize_python("write_file", &json!({ "path": "/workspace/x" }))
            .unwrap_err();
        assert!(err.contains("content"));
    }

    #[test]
    fn list_files_defaults_to_workspace() {
        let py = synthesize_python("list_files", &json!({})).unwrap();
        assert!(py.contains(r#"_resolve("/workspace")"#));
    }

    #[test]
    fn project_result_unpacks_success_envelope() {
        let exec = nanolambda_runtime::ExecutionResult {
            success: true,
            result: Some(r#"{"stdout":"hi","stderr":"","exit_code":0}"#.into()),
            error: None,
            metrics: nanolambda_runtime::ExecutionMetrics {
                cold_start_ms: 0,
                execution_ms: 42,
                total_ms: 42,
                memory_peak_mb: 0.0,
                cpu_percent: 0.0,
                exit_code: 0,
                stdout: String::new(),
                stderr: String::new(),
                is_cold_start: true,
                python_version: "3.12".into(),
            },
        };
        let projected = project_result(exec);
        assert_eq!(projected.stdout, "hi");
        assert_eq!(projected.exit_code, 0);
        assert_eq!(projected.duration_ms, 42);
        assert!(projected.cold_start);
    }

    #[test]
    fn project_result_surfaces_errors_as_stderr() {
        let exec = nanolambda_runtime::ExecutionResult {
            success: false,
            result: None,
            error: Some("boom".into()),
            metrics: nanolambda_runtime::ExecutionMetrics {
                cold_start_ms: 0,
                execution_ms: 1,
                total_ms: 1,
                memory_peak_mb: 0.0,
                cpu_percent: 0.0,
                exit_code: 0,
                stdout: String::new(),
                stderr: String::new(),
                is_cold_start: false,
                python_version: "3.12".into(),
            },
        };
        let projected = project_result(exec);
        assert_eq!(projected.stderr, "boom");
        assert_eq!(projected.exit_code, 1);
    }
}
