"""Wrap NanoLambda as a LangChain Tool."""

from langchain.tools import Tool

from nanolambda import NanoLambda

sandbox = NanoLambda(api_key="nl_...")


def run_python(code: str) -> str:
    """Execute Python code in a secure sandbox and return the output."""
    result = sandbox.execute_python(code)
    return result.stdout if result.exit_code == 0 else f"Error: {result.stderr}"


python_sandbox = Tool(
    name="python_sandbox",
    func=run_python,
    description="Execute Python code in a secure sandbox. Input is Python source code.",
)
