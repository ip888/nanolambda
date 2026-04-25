# NanoLambda

Secure code execution for AI assistants and automation.

NanoLambda gives your team a safe place to run AI-generated code without exposing production systems. It is built for fast, isolated execution of Python and shell tasks through a simple API, MCP integration, and Python SDK.

## What Is NanoLambda

NanoLambda is a sandbox platform.

- Your app or AI assistant sends code to NanoLambda
- NanoLambda runs that code in an isolated environment
- NanoLambda returns output, errors, and execution timing

This lets teams ship AI features faster while keeping security controls in place.

## Who It Is For

- Product teams adding AI-powered workflows
- Internal automation teams that run generated scripts
- Organizations that need safer execution boundaries for untrusted code
- Customers who want either managed cloud or self-hosted deployment

## Core Benefits

- Safe execution with isolation controls
- Fast execution with warm workers
- Easy integration (REST API, MCP, Python SDK)
- Observability via health and Prometheus metrics
- Self-hosting support for private environments

## Key Features

- Isolated sandbox execution
- Path and filesystem safety checks
- Memory and timeout limits per execution
- Network isolation controls
- Prometheus metrics endpoint
- MCP server for AI clients
- Python SDK with framework examples

## Architecture In Simple Terms

NanoLambda has four main blocks:

1. API Layer
Receives requests from customers, apps, or AI tools.

2. Security and Control Layer
Validates requests and applies limits (time, memory, isolation rules).

3. Sandbox Runtime
Executes code in isolated workers and captures stdout, stderr, and exit codes.

4. Storage and Monitoring
Tracks data and exposes health/metrics for operations.

### Request Flow

Client -> NanoLambda API -> Isolated Sandbox Runtime -> Result + Metrics

## Quick Start

### Option A: Use a deployed instance

Set environment variables:

```bash
export BASE_URL="https://your-instance.example.com"
export API_KEY="nl_your_key_here"
```

Health check:

```bash
curl -sS "$BASE_URL/health"
```

Create an API key (bootstrap):

```bash
API_KEY=$(curl -sS -X POST "$BASE_URL/auth/keys" \
  -H "Content-Type: application/json" \
  -d '{"name":"customer-demo","permissions":["sandbox:invoke"],"expires_at":null}' \
  | sed -n 's/.*"key":"\([^"]*\)".*/\1/p')
```

Sandbox execution:

```bash
curl -sS -X POST "$BASE_URL/sandbox/invoke" \
  -H "Authorization: Bearer $API_KEY" \
  -H "Content-Type: application/json" \
  -d '{"tool":"execute_python","args":{"code":"print(2+2)"}}'
```

Metrics check:

```bash
curl -sS "$BASE_URL/metrics/prometheus" | head -20
```

### Option B: Self-host with Docker

```bash
docker run -d -p 8080:8080 ghcr.io/ip888/nanolambda:latest
```

Then test locally:

```bash
curl -sS http://localhost:8080/health
```

## Demo Walkthrough for Customers

Use the ready-to-run demo script:

```bash
bash scripts/demo-production.sh
```

The script validates:

- service health
- metrics availability
- sandbox execution with deterministic examples

You can set BASE_URL and API_KEY first to run against production.

## Customer UI

Open the built-in customer console at `/dashboard`.

It supports:

- connection setup (BASE_URL + API key)
- API key creation and revocation
- deterministic examples for Python and shell sandbox tools
- custom sandbox execution playground

## Documentation

- Product value: docs/WHY_NANOLAMBDA.md
- Getting started: docs/QUICKSTART.md
- Python SDK: sdks/python/README.md
- Server docs: server/docs/

## License

MIT. See LICENSE.
