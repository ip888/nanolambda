"""Wrap NanoLambda as a CrewAI tool."""

from crewai.tools import tool

from nanolambda import NanoLambda

sandbox = NanoLambda(api_key="nl_...")


@tool("python_sandbox")
def python_sandbox(code: str) -> str:
    """Execute Python code in a secure NanoLambda sandbox.

    Use this to run arbitrary Python code safely. Returns stdout on success
    or stderr on failure.
    """
    result = sandbox.execute_python(code)
    return result.stdout if result.exit_code == 0 else f"Error: {result.stderr}"
