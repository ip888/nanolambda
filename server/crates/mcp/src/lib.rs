//! NanoLambda MCP server library.
//!
//! Implements a minimal, production-quality Model Context Protocol server that
//! exposes NanoLambda sandboxes as tools over stdio. Protocol reference:
//! <https://modelcontextprotocol.io/specification/2025-06-18>.
//!
//! The server speaks JSON-RPC 2.0. It handles the three required MCP methods
//! (`initialize`, `tools/list`, `tools/call`) plus a `shutdown` no-op, and
//! forwards tool invocations to a NanoLambda control plane via HTTP.

pub mod protocol;
pub mod sandbox;
pub mod server;
pub mod tools;

pub use protocol::{JsonRpcRequest, JsonRpcResponse, McpError};
pub use sandbox::{SandboxClient, SandboxResult};
pub use server::{Config, Server};
