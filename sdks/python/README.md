# NanoLambda Python SDK

Python client for the [NanoLambda](https://nanolambda.io) AI-agent code-execution sandbox.

## Install

```bash
pip install nanolambda
```

## Quickstart

```python
from nanolambda import NanoLambda

sandbox = NanoLambda(api_key="nl_...")

result = sandbox.execute_python("print('Hello from the sandbox!')")
print(result.stdout)       # Hello from the sandbox!
print(result.duration_ms)  # 12
```

## API Reference

### `NanoLambda(api_key, base_url="https://api.nanolambda.io", timeout=60.0)`

Create a client. Use as a context manager for automatic cleanup:

```python
with NanoLambda(api_key="nl_...") as sandbox:
    sandbox.execute_python("print(1+1)")
```

### Methods

| Method | Description |
|--------|-------------|
| `execute_python(code, timeout_ms=30000)` | Run Python code in the sandbox |
| `execute_shell(command)` | Run a shell command |
| `read_file(path)` | Read a file from the sandbox |
| `write_file(path, content)` | Write a file to the sandbox |
| `list_files(path="/workspace")` | List directory contents |

All methods return a `SandboxResult`.

### `SandboxResult`

| Field | Type | Description |
|-------|------|-------------|
| `stdout` | `str` | Standard output |
| `stderr` | `str` | Standard error |
| `exit_code` | `int` | Process exit code |
| `duration_ms` | `int` | Execution time in milliseconds |
| `cold_start` | `bool` | Whether a new sandbox was created |

### `NanoLambdaError`

Raised on API errors. Has a `status_code` attribute when the error comes from an HTTP response.

## License

Apache-2.0
