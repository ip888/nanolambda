//! Function registry (DEPRECATED - use StorageManager instead)
//!
//! This module is kept for backward compatibility but is superseded
//! by the StorageManager which uses SQLite for better querying.

use serde::{Deserialize, Serialize};

#[deprecated(since = "0.1.0", note = "Use StorageManager instead")]
pub struct FunctionRegistry;

#[derive(Debug, Serialize, Deserialize)]
pub struct FunctionMetadata {
    pub name: String,
    pub runtime: String,
    pub handler: String,
    pub memory_mb: u32,
    pub timeout_sec: u32,
    pub code_sha256: String,
}
