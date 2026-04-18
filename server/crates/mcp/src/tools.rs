//! Tool definitions the MCP server advertises.
//!
//! Each tool is declared once in [`descriptors`] so that the `tools/list`
//! response stays in sync with the dispatch table in [`crate::server`]. Input
//! schemas are JSON Schema draft-07 — the subset every MCP client understands.

use serde_json::json;

use crate::protocol::ToolDescriptor;

pub const EXECUTE_PYTHON: &str = "execute_python";
pub const EXECUTE_SHELL: &str = "execute_shell";
pub const READ_FILE: &str = "read_file";
pub const WRITE_FILE: &str = "write_file";
pub const LIST_FILES: &str = "list_files";

/// Catalog of every tool this MCP server exposes.
///
/// Keep the descriptions short and action-oriented. Agent planners use them
/// as the primary signal for tool selection — long prose measurably hurts
/// dispatch accuracy on smaller models.
pub fn descriptors() -> Vec<ToolDescriptor> {
    vec![
        ToolDescriptor {
            name: EXECUTE_PYTHON,
            description: "Execute Python 3.11 code in an isolated NanoLambda sandbox. \
                Returns stdout, stderr, and exit code. Network access is disabled by default.",
            input_schema: json!({
                "type": "object",
                "properties": {
                    "code": {
                        "type": "string",
                        "description": "Python source to execute. Prefer self-contained snippets."
                    },
                    "stdin": {
                        "type": "string",
                        "description": "Optional stdin payload forwarded to the process.",
                        "default": ""
                    },
                    "timeout_ms": {
                        "type": "integer",
                        "minimum": 100,
                        "maximum": 300_000,
                        "description": "Wall-clock limit. Defaults to the server-side setting."
                    }
                },
                "required": ["code"],
                "additionalProperties": false
            }),
        },
        ToolDescriptor {
            name: EXECUTE_SHELL,
            description: "Run a shell command inside the sandbox. Use for POSIX \
                utilities (ls, cat, jq, curl). One command per call.",
            input_schema: json!({
                "type": "object",
                "properties": {
                    "command": {
                        "type": "string",
                        "description": "Command line, passed to /bin/sh -c."
                    },
                    "cwd": {
                        "type": "string",
                        "description": "Working directory. Defaults to /workspace."
                    },
                    "timeout_ms": {
                        "type": "integer",
                        "minimum": 100,
                        "maximum": 300_000
                    }
                },
                "required": ["command"],
                "additionalProperties": false
            }),
        },
        ToolDescriptor {
            name: READ_FILE,
            description: "Read a UTF-8 file from the sandbox filesystem.",
            input_schema: json!({
                "type": "object",
                "properties": {
                    "path": {
                        "type": "string",
                        "description": "Absolute or workspace-relative path."
                    }
                },
                "required": ["path"],
                "additionalProperties": false
            }),
        },
        ToolDescriptor {
            name: WRITE_FILE,
            description: "Write UTF-8 contents to the sandbox filesystem, creating \
                parent directories as needed.",
            input_schema: json!({
                "type": "object",
                "properties": {
                    "path": { "type": "string" },
                    "content": { "type": "string" }
                },
                "required": ["path", "content"],
                "additionalProperties": false
            }),
        },
        ToolDescriptor {
            name: LIST_FILES,
            description: "List entries in a sandbox directory (non-recursive).",
            input_schema: json!({
                "type": "object",
                "properties": {
                    "path": {
                        "type": "string",
                        "default": "/workspace"
                    }
                },
                "additionalProperties": false
            }),
        },
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn all_descriptors_have_object_schema() {
        for d in descriptors() {
            assert_eq!(d.input_schema["type"], "object");
            assert!(!d.name.is_empty());
            assert!(!d.description.is_empty());
        }
    }

    #[test]
    fn descriptor_names_are_unique() {
        let mut names: Vec<_> = descriptors().iter().map(|d| d.name).collect();
        let original = names.len();
        names.sort_unstable();
        names.dedup();
        assert_eq!(names.len(), original);
    }
}
