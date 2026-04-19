# Why NanoLambda

## The Problem

AI agents are writing and executing code at an unprecedented scale. Every LLM-powered
workflow that generates Python, shell commands, or data-analysis scripts needs a place to
run that code. Running it on your own infrastructure is a security risk. Running it on
general-purpose cloud functions is slow, expensive, and operationally complex.

Engineering teams need a sandbox that is:

- **Secure** -- untrusted code must never reach production systems
- **Fast** -- agents are interactive; users will not wait 5 seconds for a cold start
- **Simple** -- integration should take less than an hour, not a sprint
- **Predictable** -- costs should scale linearly with usage, with no surprises

## The Solution

NanoLambda is a purpose-built code-execution sandbox for AI agents. Every invocation runs
in an isolated environment with strict resource limits. There is nothing to configure,
no infrastructure to manage, and no vendor lock-in.

```python
from nanolambda import NanoLambda

sandbox = NanoLambda(api_key="nl_...")
result = sandbox.execute_python("print(2 + 2)")
# result.stdout == "4\n", result.duration_ms == 11
```

## How NanoLambda Compares

| Capability | NanoLambda | E2B | Modal | AWS Lambda |
|---|---|---|---|---|
| Cold start | < 50 ms | ~500 ms | ~1 s | 1-5 s |
| Purpose-built for AI agents | Yes | Yes | No | No |
| MCP integration | Native | No | No | No |
| Self-host option | Yes | No | No | No |
| Per-invocation billing | Yes | Yes | Yes | Yes |
| Minimum monthly cost | $0 | $0 | $30 | $0 |
| Open source | MIT | Partial | No | No |
| Process isolation | Yes | Yes | Yes | Yes |
| Network isolation | Configurable | Configurable | No | VPC required |
| Memory limits | Per-sandbox | Per-sandbox | Per-function | Per-function |

## Security Model

Every sandbox execution is isolated at multiple layers:

1. **Process isolation** -- each execution runs in its own process with a dedicated
   user ID; there is no shared state between invocations.
2. **Memory limits** -- each sandbox is capped at a configurable memory ceiling
   (default 256 MB). Exceeding it terminates the process immediately.
3. **CPU time limits** -- runaway loops and crypto miners are killed after the
   configured timeout (default 30 seconds).
4. **Network isolation** -- outbound network access is disabled by default. It can be
   enabled per-sandbox when your use case requires it (e.g., fetching a dataset).
5. **Filesystem isolation** -- each sandbox gets a temporary filesystem that is
   destroyed after execution. No data persists unless explicitly saved.
6. **No privilege escalation** -- sandbox processes run as unprivileged users with no
   access to host resources.

## Pricing

NanoLambda uses straightforward, usage-based pricing with no minimums and no commitments.

| Tier | Invocations / month | Price per 1,000 invocations |
|---|---|---|
| Free | Up to 10,000 | $0 |
| Growth | 10,001 - 1,000,000 | $0.50 |
| Scale | 1,000,001+ | $0.25 |

Self-hosted deployments are free and unlimited under the MIT license.

## Integration

NanoLambda integrates with your stack in minutes, not days.

**MCP (Model Context Protocol)** -- add a JSON block to your Claude Desktop or Cursor
config file. Your AI assistant can immediately execute code in a sandbox.

**Python SDK** -- `pip install nanolambda` and call `sandbox.execute_python(code)`.
Framework adapters are included for LangChain, CrewAI, and Pydantic-AI.

**REST API** -- a single `POST /sandbox/invoke` endpoint accepts JSON and returns
structured results. Works with any language or framework.

**Typical integration time: under 1 hour.**

## Self-Hosting

For regulated industries -- banking, healthcare, government -- NanoLambda can run entirely
on your own infrastructure. The server is a single binary that deploys to any Linux host
or Docker environment.

Self-hosted benefits:

- Data never leaves your network
- Full control over resource limits and network policies
- No usage fees
- Same API surface as the cloud version
- Deploy to air-gapped environments

```bash
docker run -d -p 8080:8080 ghcr.io/ip888/nanolambda:latest
```

## Who Uses NanoLambda

NanoLambda is built for teams that are integrating code execution into AI-powered products:

- **AI application developers** building agents that write and run code
- **Data platforms** offering AI-assisted analysis to their users
- **Developer tools** adding AI code execution to IDEs and notebooks
- **Enterprises** deploying internal AI assistants that need safe code execution

## Get Started

1. Sign up at [nanolambda.io](https://nanolambda.io) or self-host with Docker
2. Follow the [Quickstart guide](QUICKSTART.md)
3. Browse the [Python SDK documentation](../sdks/python/README.md)

Questions? Reach out at support@nanolambda.io.
