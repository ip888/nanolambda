use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::str::FromStr;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FunctionConfig {
    pub name: String,
    pub runtime: String,
    pub handler: String,
    pub code: String,
    pub memory_mb: u64,
    pub timeout_ms: u64,
    #[serde(default)]
    pub environment: HashMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Function {
    pub id: i64,
    pub name: String,
    pub runtime: String,
    pub handler: String,
    pub code: String,
    pub code_hash: String,
    pub memory_mb: u64,
    pub timeout_ms: u64,
    pub environment: HashMap<String, String>,
    pub created_at: i64,
    pub updated_at: i64,
    pub last_invoked_at: Option<i64>,
    pub invocation_count: i64,
    pub total_execution_time_ms: i64,
    pub status: FunctionStatus,
    pub version: i64,
    pub is_latest: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum FunctionStatus {
    Active,
    Disabled,
    Deleted,
}

impl FunctionStatus {
    pub fn as_str(&self) -> &str {
        match self {
            FunctionStatus::Active => "active",
            FunctionStatus::Disabled => "disabled",
            FunctionStatus::Deleted => "deleted",
        }
    }
}

impl FromStr for FunctionStatus {
    type Err = ();

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        Ok(match s {
            "active" => FunctionStatus::Active,
            "disabled" => FunctionStatus::Disabled,
            "deleted" => FunctionStatus::Deleted,
            _ => FunctionStatus::Active,
        })
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InvocationRecord {
    pub function_id: i64,
    pub request_id: String,
    pub status: InvocationStatus,
    pub started_at: i64,
    pub completed_at: Option<i64>,
    pub execution_time_ms: Option<i64>,
    pub memory_used_mb: Option<i64>,
    pub cold_start: bool,
    pub error_message: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum InvocationStatus {
    Success,
    Error,
    Timeout,
}

impl InvocationStatus {
    pub fn as_str(&self) -> &str {
        match self {
            InvocationStatus::Success => "success",
            InvocationStatus::Error => "error",
            InvocationStatus::Timeout => "timeout",
        }
    }
}

impl FromStr for InvocationStatus {
    type Err = ();

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        Ok(match s {
            "success" => InvocationStatus::Success,
            "error" => InvocationStatus::Error,
            "timeout" => InvocationStatus::Timeout,
            _ => InvocationStatus::Error,
        })
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FunctionStats {
    pub invocation_count: i64,
    pub total_execution_time_ms: i64,
    pub avg_execution_time_ms: f64,
    pub cold_start_count: i64,
    pub error_count: i64,
    pub timeout_count: i64,
    pub last_invoked_at: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiKey {
    pub id: i64,
    pub key: String,
    pub name: String,
    pub permissions: Vec<String>,
    pub created_at: i64,
    pub expires_at: Option<i64>,
    pub status: ApiKeyStatus,
    pub last_used_at: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum ApiKeyStatus {
    Active,
    Revoked,
}

impl ApiKeyStatus {
    pub fn as_str(&self) -> &str {
        match self {
            ApiKeyStatus::Active => "active",
            ApiKeyStatus::Revoked => "revoked",
        }
    }
}

impl FromStr for ApiKeyStatus {
    type Err = ();

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        Ok(match s {
            "active" => ApiKeyStatus::Active,
            "revoked" => ApiKeyStatus::Revoked,
            _ => ApiKeyStatus::Active,
        })
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateApiKeyRequest {
    pub name: String,
    #[serde(default)]
    pub permissions: Vec<String>,
    pub expires_at: Option<i64>,
}
