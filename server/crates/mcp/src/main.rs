//! nanolambda-mcp binary entry point.
//!
//! Runs a Model Context Protocol (MCP) server over stdio that exposes
//! NanoLambda sandboxes as tools. Any MCP-compatible agent host (Claude
//! Desktop, Cursor, Zed, Windsurf, the OpenAI Responses API, etc.) can
//! register this binary and call `execute_python`, `execute_shell`,
//! `read_file`, `write_file`, and `list_files` against an isolated
//! NanoLambda sandbox.

use clap::Parser;
use nanolambda_mcp::{Config, Server};
use tracing_subscriber::{EnvFilter, fmt};

#[derive(Debug, Parser)]
#[command(
    name = "nanolambda-mcp",
    version,
    about = "MCP server that exposes NanoLambda sandboxes to AI agents"
)]
struct Cli {
    /// NanoLambda control-plane base URL.
    #[arg(long, env = "NANOLAMBDA_URL", default_value = "http://127.0.0.1:3000")]
    url: String,

    /// API key for the NanoLambda control plane.
    #[arg(long, env = "NANOLAMBDA_API_KEY")]
    api_key: Option<String>,

    /// Per-sandbox wall-clock timeout in milliseconds.
    #[arg(long, env = "NANOLAMBDA_TIMEOUT_MS", default_value_t = 30_000)]
    timeout_ms: u64,

    /// Memory cap per sandbox, in megabytes.
    #[arg(long, env = "NANOLAMBDA_MEMORY_MB", default_value_t = 256)]
    memory_mb: u32,
}

#[tokio::main(flavor = "current_thread")]
async fn main() -> anyhow::Result<()> {
    // Logs go to stderr so they never collide with the JSON-RPC stream on stdout.
    fmt()
        .with_env_filter(
            EnvFilter::try_from_env("NANOLAMBDA_MCP_LOG")
                .unwrap_or_else(|_| EnvFilter::new("info")),
        )
        .with_writer(std::io::stderr)
        .with_target(false)
        .init();

    let cli = Cli::parse();
    let config = Config {
        base_url: cli.url,
        api_key: cli.api_key,
        timeout_ms: cli.timeout_ms,
        memory_mb: cli.memory_mb,
    };

    tracing::info!(url = %config.base_url, "nanolambda-mcp starting");
    Server::new(config).run().await
}
