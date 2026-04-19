# nanolambda-mcp

Model Context Protocol (MCP) server that exposes a NanoLambda sandbox to any
MCP-compatible agent host: Claude Desktop, Cursor, Zed, Windsurf, the OpenAI
Responses API, LangChain MCP adapters, and the `mcp-inspector` CLI.

## Why

MCP is the de-facto way to give an LLM agent tool access in 2026. This crate
ships a single static binary (`nanolambda-mcp`) that speaks the 2025-06-18
protocol revision and forwards tool calls to a NanoLambda control plane. Drop
the binary into any agent host config and the agent immediately has isolated
Python execution, shell access, and filesystem I/O.

## Tools exposed

| Tool             | Purpose                                              |
| ---------------- | ---------------------------------------------------- |
| `execute_python` | Run Python 3.12+ source in an isolated sandbox        |
| `execute_shell`  | Run a single shell command via `/bin/sh -c`          |
| `read_file`      | Read a UTF-8 file from the sandbox FS                |
| `write_file`     | Write UTF-8 content, creating parent dirs as needed  |
| `list_files`     | List entries in a sandbox directory (non-recursive)  |

## Usage

```sh
NANOLAMBDA_URL=https://api.nanolambda.example \
NANOLAMBDA_API_KEY=nl_live_... \
nanolambda-mcp
```

### Claude Desktop (`claude_desktop_config.json`)

```json
{
  "mcpServers": {
    "nanolambda": {
      "command": "nanolambda-mcp",
      "env": {
        "NANOLAMBDA_URL": "https://api.nanolambda.example",
        "NANOLAMBDA_API_KEY": "nl_live_..."
      }
    }
  }
}
```

### mcp-inspector

```sh
npx @modelcontextprotocol/inspector nanolambda-mcp
```

## Wire protocol

Newline-delimited JSON-RPC 2.0 over stdio. Methods handled:

- `initialize` → advertises server info + `tools` capability
- `tools/list` → returns the catalog in [`tools.rs`](src/tools.rs)
- `tools/call` → forwards to `POST {NANOLAMBDA_URL}/sandbox/invoke`
- `notifications/initialized`, `shutdown`, `exit`, `ping` → handled as no-ops

stderr carries `tracing` logs; stdout is reserved for the JSON-RPC stream.

## License

MIT.
