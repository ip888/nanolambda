# Changelog

All notable changes to NanoLambda will be documented in this file.

## [Unreleased]

### Added
- Sandbox execution API (`/v1/sandbox/execute`)
- Python 3.11, 3.12, and 3.13 support with CI matrix
- MCP server (`nanolambda-mcp`) for Claude Desktop / Cursor integration
- Python SDK with LangChain, CrewAI, Pydantic-AI examples
- Prometheus metrics endpoint (`/metrics/prometheus`)
- OS-level sandbox isolation (network namespace, memory limits, path sandboxing)
- Pre-warmed process pool for sub-millisecond warm starts
- Pre-push and pre-deploy check scripts
- Fly.io deployment configuration
- Marketing landing page and use-case pages

### Security
- Path traversal fix: `os.path.realpath` + prefix check in sandbox
- Network isolation: `CLONE_NEWNET` namespace per execution
- Process pool cache collision fix: unique IDs per invocation

### Removed
- Edge platform (Cloudflare Workers) — consolidated to single-server architecture
- Kubernetes manifests — using Fly.io instead
- DigitalOcean deployment workflow — replaced with Fly.io

[Unreleased]: https://github.com/ip888/nanolambda/compare/v0.1.0...HEAD
