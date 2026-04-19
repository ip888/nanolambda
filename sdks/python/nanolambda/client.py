"""NanoLambda Python SDK — thin client for the AI-agent sandbox API."""

from __future__ import annotations

import json
from dataclasses import dataclass
from typing import Any

import httpx


class NanoLambdaError(Exception):
    """Raised when the NanoLambda API returns an error."""

    def __init__(self, message: str, status_code: int | None = None):
        super().__init__(message)
        self.status_code = status_code


@dataclass(frozen=True, slots=True)
class SandboxResult:
    """Structured result from a sandbox execution."""

    stdout: str
    stderr: str
    exit_code: int
    duration_ms: int
    cold_start: bool

    @classmethod
    def from_response(cls, data: dict[str, Any]) -> SandboxResult:
        return cls(
            stdout=data.get("stdout", ""),
            stderr=data.get("stderr", ""),
            exit_code=data.get("exit_code", -1),
            duration_ms=data.get("duration_ms", 0),
            cold_start=data.get("cold_start", False),
        )


class NanoLambda:
    """Client for the NanoLambda sandbox API.

    Usage::

        from nanolambda import NanoLambda

        sandbox = NanoLambda(api_key="nl_...")
        result = sandbox.execute_python("print('hello')")
        print(result.stdout)
    """

    def __init__(
        self,
        api_key: str,
        base_url: str = "https://api.nanolambda.io",
        timeout: float = 60.0,
    ):
        self.api_key = api_key
        self.base_url = base_url.rstrip("/")
        self._client = httpx.Client(
            base_url=self.base_url,
            headers={
                "Authorization": f"Bearer {api_key}",
                "Content-Type": "application/json",
            },
            timeout=timeout,
        )

    # -- public API ----------------------------------------------------------

    def execute_python(self, code: str, timeout_ms: int = 30_000) -> SandboxResult:
        """Execute Python code in a sandboxed environment."""
        return self._invoke(
            {"action": "execute_python", "code": code, "timeout_ms": timeout_ms}
        )

    def execute_shell(self, command: str) -> SandboxResult:
        """Execute a shell command in the sandbox."""
        return self._invoke({"action": "execute_shell", "command": command})

    def read_file(self, path: str) -> SandboxResult:
        """Read a file from the sandbox filesystem."""
        return self._invoke({"action": "read_file", "path": path})

    def write_file(self, path: str, content: str) -> SandboxResult:
        """Write content to a file in the sandbox filesystem."""
        return self._invoke(
            {"action": "write_file", "path": path, "content": content}
        )

    def list_files(self, path: str = "/workspace") -> SandboxResult:
        """List files in a sandbox directory."""
        return self._invoke({"action": "list_files", "path": path})

    # -- internals -----------------------------------------------------------

    def _invoke(self, payload: dict[str, Any]) -> SandboxResult:
        try:
            resp = self._client.post("/sandbox/invoke", json=payload)
        except httpx.HTTPError as exc:
            raise NanoLambdaError(f"HTTP request failed: {exc}") from exc

        if resp.status_code >= 400:
            try:
                detail = resp.json().get("error", resp.text)
            except (json.JSONDecodeError, ValueError):
                detail = resp.text
            raise NanoLambdaError(detail, status_code=resp.status_code)

        return SandboxResult.from_response(resp.json())

    def close(self) -> None:
        """Close the underlying HTTP client."""
        self._client.close()

    def __enter__(self) -> NanoLambda:
        return self

    def __exit__(self, *args: Any) -> None:
        self.close()
