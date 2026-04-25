# NanoLambda Quickstart

Get a sandbox running in under 3 minutes.

## 1. Get an API key

**Cloud** (fastest): Sign up at [nanolambda.io](https://nanolambda.io) and copy your key from the dashboard.

**Self-host**:

```bash
docker run -d -p 8080:8080 ghcr.io/ip888/nanolambda:latest
# Your base URL is http://localhost:8080 — no API key required in local mode
```

## 2. Choose your integration

### Option A: MCP (Claude Desktop / Cursor / any MCP client)

Add this to your Claude Desktop config (`~/Library/Application Support/Claude/claude_desktop_config.json`):

```json
{
  "mcpServers": {
    "nanolambda": {
      "command": "nanolambda-mcp",
      "env": {
        "NANOLAMBDA_URL": "http://localhost:8080",
        "NANOLAMBDA_API_KEY": "nl_..."
      }
    }
  }
}
```

Restart Claude Desktop. You can now ask Claude to run code and it will execute in a sandbox.

### Option B: Python SDK

```bash
pip install nanolambda
```

```python
from nanolambda import NanoLambda

sandbox = NanoLambda(api_key="nl_...")

result = sandbox.execute_python("import math; print(math.pi)")
print(result.stdout)       # 3.141592653589793
print(result.duration_ms)  # 11
```

## 3. Reproducible production demo

Run this from the repository root:

```bash
export BASE_URL="https://your-instance.example.com"
export API_KEY="nl_..."
bash scripts/demo-production.sh
```

What it verifies:

- `/health` returns HTTP 200
- `/metrics/prometheus` is reachable and contains expected metrics
- Two deterministic sandbox executions return expected outputs (`4` and `60`)

## 4. What's next

- Browse framework examples: [LangChain](../sdks/python/examples/langchain_tool.py), [CrewAI](../sdks/python/examples/crewai_tool.py), [Pydantic-AI](../sdks/python/examples/pydantic_ai_tool.py)
- Read the [Python SDK docs](../sdks/python/README.md)
- Check execution metrics at `/metrics/prometheus` (Prometheus format)
- Read [Why NanoLambda](WHY_NANOLAMBDA.md) for the full value proposition
