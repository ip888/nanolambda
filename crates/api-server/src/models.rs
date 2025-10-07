//! API data models

use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct Function {
    pub name: String,
    pub runtime: Runtime,
    pub handler: String,
    pub memory_mb: u32,
    pub timeout_sec: u32,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Runtime {
    #[serde(rename = "python3.11")]
    Python311,
    #[serde(rename = "nodejs20.x")]
    NodeJs20,
    #[serde(rename = "java21")]
    Java21,
}
