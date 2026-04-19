"""Unit tests for the NanoLambda Python SDK."""

from unittest.mock import MagicMock, patch

import pytest

from nanolambda.client import NanoLambda, NanoLambdaError, SandboxResult


# -- SandboxResult -----------------------------------------------------------

def test_sandbox_result_from_response():
    data = {
        "stdout": "hello\n",
        "stderr": "",
        "exit_code": 0,
        "duration_ms": 42,
        "cold_start": True,
    }
    result = SandboxResult.from_response(data)
    assert result.stdout == "hello\n"
    assert result.exit_code == 0
    assert result.cold_start is True


def test_sandbox_result_defaults():
    result = SandboxResult.from_response({})
    assert result.stdout == ""
    assert result.exit_code == -1
    assert result.cold_start is False


# -- Client construction -----------------------------------------------------

def test_client_construction():
    client = NanoLambda(api_key="nl_test", base_url="http://localhost:8080")
    assert client.api_key == "nl_test"
    assert client.base_url == "http://localhost:8080"
    client.close()


def test_client_strips_trailing_slash():
    client = NanoLambda(api_key="nl_test", base_url="http://localhost:8080/")
    assert client.base_url == "http://localhost:8080"
    client.close()


# -- execute_python -----------------------------------------------------------

def _mock_response(json_data: dict, status_code: int = 200) -> MagicMock:
    resp = MagicMock()
    resp.status_code = status_code
    resp.json.return_value = json_data
    resp.text = str(json_data)
    return resp


@patch("nanolambda.client.httpx.Client")
def test_execute_python_sends_correct_payload(mock_client_cls):
    mock_http = MagicMock()
    mock_client_cls.return_value = mock_http
    mock_http.post.return_value = _mock_response(
        {"stdout": "42\n", "stderr": "", "exit_code": 0, "duration_ms": 15, "cold_start": False}
    )

    client = NanoLambda(api_key="nl_test")
    result = client.execute_python("print(42)")

    mock_http.post.assert_called_once_with(
        "/sandbox/invoke",
        json={"action": "execute_python", "code": "print(42)", "timeout_ms": 30_000},
    )
    assert result.stdout == "42\n"
    assert result.exit_code == 0


# -- Error handling -----------------------------------------------------------

@patch("nanolambda.client.httpx.Client")
def test_api_error_raises_nanolambda_error(mock_client_cls):
    mock_http = MagicMock()
    mock_client_cls.return_value = mock_http
    mock_http.post.return_value = _mock_response(
        {"error": "unauthorized"}, status_code=401
    )

    client = NanoLambda(api_key="bad_key")
    with pytest.raises(NanoLambdaError) as exc_info:
        client.execute_python("print(1)")

    assert exc_info.value.status_code == 401
    assert "unauthorized" in str(exc_info.value)


# -- Context manager ----------------------------------------------------------

@patch("nanolambda.client.httpx.Client")
def test_context_manager(mock_client_cls):
    mock_http = MagicMock()
    mock_client_cls.return_value = mock_http
    mock_http.post.return_value = _mock_response(
        {"stdout": "", "stderr": "", "exit_code": 0, "duration_ms": 5, "cold_start": False}
    )

    with NanoLambda(api_key="nl_test") as client:
        client.execute_python("pass")

    mock_http.close.assert_called_once()
