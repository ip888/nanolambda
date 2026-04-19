# NanoLambda

> **The fastest AI-agent code-execution sandbox — self-hosted, MCP-native, sub-10ms warm starts**

[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)
[![Rust](https://img.shields.io/badge/rust-1.93+-orange.svg)](https://www.rust-lang.org/)
[![Edition](https://img.shields.io/badge/edition-2024-blue.svg)](https://doc.rust-lang.org/edition-guide/rust-2024/)

---

## What is NanoLambda?

NanoLambda lets AI agents execute Python code in a secure sandbox via a single API call. Purpose-built for LLM tool-use, MCP servers, and AI agent frameworks.

```bash
curl -X POST https://api.nanolambda.io/v1/sandbox/execute \
  -H "Authorization: Bearer nl_..." \
  -d '{"code": "print(2+2)", "runtime": "python"}'
# → {"stdout": "4\n", "exit_code": 0, "duration_ms": 3}
```

## Features

- **~0ms warm starts** — pre-warmed process pool, 10-50x faster than AWS Lambda
- **~32ms cold starts** — 3-10x faster than competitors
- **OS-level isolation** — network namespace, memory limits, path sandboxing
- **MCP server** — `nanolambda-mcp` binary for Claude Desktop, Cursor, any MCP client
- **Python SDK** — with LangChain, CrewAI, Pydantic-AI integrations
- **Prometheus metrics** — `/metrics/prometheus` endpoint
- **Self-hostable** — single Docker container, MIT licensed
- **Multi-Python** — supports Python 3.12 and 3.13

## Quick Start

**Docker (fastest):**
```bash
docker run -d -p 8080:8080 ghcr.io/ip888/nanolambda/nanolambda-server:latest
```

**From source:**
```bash
cd server
cargo build --release
cargo run --release --bin nanolambda-server
```

See [docs/QUICKSTART.md](docs/QUICKSTART.md) for MCP, SDK, and framework integration guides.

## Project Structure

```
nanolambda/
├── server/          # Rust API server + Python sandbox runtime
│   ├── crates/
│   │   ├── api-server/   # Axum HTTP API + handlers
│   │   ├── runtime/      # PythonExecutor + process pool
│   │   ├── storage/      # SQLite persistence
│   │   └── mcp/          # MCP JSON-RPC server
│   └── Dockerfile
├── sdks/python/     # Python client SDK
├── marketing/       # Landing page + use-case pages
├── docs/            # Quickstart, pitch docs
├── scripts/         # Pre-push and pre-deploy checks
└── .github/         # CI/CD workflows
```

## Development

### Prerequisites

- Linux x86_64 (macOS for development, Linux for sandbox isolation)
- Rust 1.93+
- Python 3.12+

### Code Quality

- **Rust Edition 2024** with `rust-version = "1.93"`
- **Clippy pedantic** with `-D warnings`
- **CI matrix** — tests against Python 3.12 and 3.13
- **Pre-push checks** — `./scripts/pre-push-check.sh`

### Building & Testing

```bash
cd server
cargo fmt --check       # formatting
cargo clippy --all-targets --all-features -- -D warnings
cargo test --workspace  # all tests
```

## Documentation

- [Quickstart](docs/QUICKSTART.md) — get running in 3 minutes
- [Why NanoLambda](docs/WHY_NANOLAMBDA.md) — value proposition for decision-makers
- [Python SDK](sdks/python/README.md) — client library docs
- [MCP Server](server/crates/mcp/README.md) — MCP integration guide
- [API Docs](server/docs/) — detailed server documentation

## License

MIT License — see [LICENSE](LICENSE) for details.

## Contributing

See [CONTRIBUTING.md](CONTRIBUTING.md) for contribution guidelines.
