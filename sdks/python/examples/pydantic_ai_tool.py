"""Wrap NanoLambda as a Pydantic-AI tool."""

from pydantic_ai import Agent

from nanolambda import NanoLambda

sandbox = NanoLambda(api_key="nl_...")
agent = Agent("openai:gpt-4o")


@agent.tool_plain
def run_python(code: str) -> str:
    """Execute Python code in a secure NanoLambda sandbox.

    Returns stdout on success or stderr on failure.
    """
    result = sandbox.execute_python(code)
    return result.stdout if result.exit_code == 0 else f"Error: {result.stderr}"
