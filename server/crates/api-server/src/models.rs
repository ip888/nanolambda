/// API data models
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct FunctionConfig {
    pub name: String,
    pub runtime: Runtime,
    pub handler: String,
    pub memory_mb: u32,
    pub timeout_sec: u32,
    #[serde(default)]
    pub code: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct InvocationRequest {
    #[serde(default)]
    pub payload: serde_json::Value,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct InvocationResponse {
    pub status_code: u32,
    pub body: String,
    pub error: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metrics: Option<ExecutionMetrics>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionMetrics {
    pub cold_start_ms: u64,
    pub execution_ms: u64,
    pub total_ms: u64,
    pub memory_peak_mb: f64,
    pub python_version: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Function {
    pub name: String,
    pub runtime: Runtime,
    pub handler: String,
    pub memory_mb: u32,
    pub timeout_sec: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Runtime {
    #[serde(rename = "python3.11")]
    Python311,
    #[serde(rename = "python3.12")]
    Python312,
    #[serde(rename = "nodejs20.x")]
    NodeJs20,
    #[serde(rename = "java21")]
    Java21,
}
