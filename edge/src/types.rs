//! Type definitions for NanoLambda Edge

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

// ============================================================================
// Function Types
// ============================================================================

/// Function definition stored in KV
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Function {
    pub id: String,
    pub name: String,
    pub runtime: Runtime,
    pub code: String,
    pub handler: String,
    pub timeout_ms: u64,
    pub memory_mb: u32,
    pub environment: HashMap<String, String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub version: u32,
    pub owner_id: String,
}

/// Supported runtimes
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum Runtime {
    JavaScript,
    Python,
}

impl Default for Runtime {
    fn default() -> Self {
        Runtime::JavaScript
    }
}

/// Function invocation request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InvokeRequest {
    pub payload: serde_json::Value,
    #[serde(default)]
    pub async_invoke: bool,
    #[serde(default)]
    pub timeout_ms: Option<u64>,
}

/// Function invocation response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InvokeResponse {
    pub request_id: String,
    pub status: InvokeStatus,
    pub result: Option<serde_json::Value>,
    pub error: Option<String>,
    pub duration_ms: u64,
    pub billed_duration_ms: u64,
    pub memory_used_mb: u32,
}

/// Invocation status
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum InvokeStatus {
    Success,
    Error,
    Timeout,
    Pending,
}

/// Create function request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateFunctionRequest {
    pub name: String,
    pub runtime: Runtime,
    pub code: String,
    #[serde(default = "default_handler")]
    pub handler: String,
    #[serde(default = "default_timeout")]
    pub timeout_ms: u64,
    #[serde(default = "default_memory")]
    pub memory_mb: u32,
    #[serde(default)]
    pub environment: HashMap<String, String>,
}

fn default_handler() -> String {
    "handler".to_string()
}

fn default_timeout() -> u64 {
    30000 // 30 seconds
}

fn default_memory() -> u32 {
    128
}

/// Update function request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateFunctionRequest {
    pub code: Option<String>,
    pub handler: Option<String>,
    pub timeout_ms: Option<u64>,
    pub memory_mb: Option<u32>,
    pub environment: Option<HashMap<String, String>>,
}

// ============================================================================
// Vector Types (QuartzDB)
// ============================================================================

/// Vector with metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Vector {
    pub id: String,
    pub values: Vec<f32>,
    pub metadata: HashMap<String, serde_json::Value>,
    pub namespace: String,
}

/// Vector upsert request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VectorUpsertRequest {
    pub vectors: Vec<VectorInput>,
    #[serde(default)]
    pub namespace: String,
}

/// Vector input for upsert
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VectorInput {
    pub id: String,
    pub values: Vec<f32>,
    #[serde(default)]
    pub metadata: HashMap<String, serde_json::Value>,
}

/// Vector query request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VectorQueryRequest {
    pub vector: Vec<f32>,
    #[serde(default = "default_top_k")]
    pub top_k: usize,
    #[serde(default)]
    pub namespace: String,
    #[serde(default)]
    pub filter: Option<VectorFilter>,
    #[serde(default = "default_include_metadata")]
    pub include_metadata: bool,
    #[serde(default)]
    pub include_values: bool,
}

fn default_top_k() -> usize {
    10
}

fn default_include_metadata() -> bool {
    true
}

/// Vector filter for queries
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VectorFilter {
    #[serde(rename = "$and")]
    pub and: Option<Vec<VectorFilter>>,
    #[serde(rename = "$or")]
    pub or: Option<Vec<VectorFilter>>,
    #[serde(flatten)]
    pub conditions: HashMap<String, FilterCondition>,
}

/// Filter condition operators
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum FilterCondition {
    Eq(serde_json::Value),
    Complex {
        #[serde(rename = "$eq")]
        eq: Option<serde_json::Value>,
        #[serde(rename = "$ne")]
        ne: Option<serde_json::Value>,
        #[serde(rename = "$gt")]
        gt: Option<f64>,
        #[serde(rename = "$gte")]
        gte: Option<f64>,
        #[serde(rename = "$lt")]
        lt: Option<f64>,
        #[serde(rename = "$lte")]
        lte: Option<f64>,
        #[serde(rename = "$in")]
        in_: Option<Vec<serde_json::Value>>,
        #[serde(rename = "$nin")]
        nin: Option<Vec<serde_json::Value>>,
    },
}

/// Vector query match result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VectorMatch {
    pub id: String,
    pub score: f32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub values: Option<Vec<f32>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<HashMap<String, serde_json::Value>>,
}

/// Vector query response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VectorQueryResponse {
    pub matches: Vec<VectorMatch>,
    pub namespace: String,
}

/// Vector delete request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VectorDeleteRequest {
    pub ids: Option<Vec<String>>,
    #[serde(default)]
    pub delete_all: bool,
    #[serde(default)]
    pub namespace: String,
    pub filter: Option<VectorFilter>,
}

// ============================================================================
// AI Types
// ============================================================================

/// AI embedding request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmbeddingRequest {
    pub text: EmbeddingInput,
    #[serde(default = "default_embedding_model")]
    pub model: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum EmbeddingInput {
    Single(String),
    Multiple(Vec<String>),
}

fn default_embedding_model() -> String {
    "@cf/baai/bge-base-en-v1.5".to_string()
}

/// AI embedding response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmbeddingResponse {
    pub model: String,
    pub embeddings: Vec<Vec<f32>>,
    pub usage: EmbeddingUsage,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmbeddingUsage {
    pub prompt_tokens: u32,
    pub total_tokens: u32,
}

/// AI text generation request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TextGenerationRequest {
    pub prompt: String,
    #[serde(default = "default_text_model")]
    pub model: String,
    #[serde(default)]
    pub max_tokens: Option<u32>,
    #[serde(default)]
    pub temperature: Option<f32>,
    #[serde(default)]
    pub stream: bool,
}

fn default_text_model() -> String {
    "@cf/meta/llama-2-7b-chat-int8".to_string()
}

/// AI text generation response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TextGenerationResponse {
    pub model: String,
    pub response: String,
    pub usage: Option<TextGenerationUsage>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TextGenerationUsage {
    pub prompt_tokens: u32,
    pub completion_tokens: u32,
    pub total_tokens: u32,
}

// ============================================================================
// Auth Types
// ============================================================================

/// API key metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiKey {
    pub key_hash: String,
    pub name: String,
    pub owner_id: String,
    pub permissions: Vec<Permission>,
    pub rate_limit: RateLimit,
    pub created_at: DateTime<Utc>,
    pub expires_at: Option<DateTime<Utc>>,
    pub last_used_at: Option<DateTime<Utc>>,
    pub is_active: bool,
}

/// Permission types
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum Permission {
    FunctionsRead,
    FunctionsWrite,
    FunctionsInvoke,
    VectorsRead,
    VectorsWrite,
    AiAccess,
    AIRead,
    AIWrite,
    CacheRead,
    CacheWrite,
    SearchRead,
    SearchWrite,
    Admin,
}

/// Rate limit configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RateLimit {
    pub requests_per_minute: u32,
    pub requests_per_day: u32,
    pub concurrent_invocations: u32,
}

impl Default for RateLimit {
    fn default() -> Self {
        Self {
            requests_per_minute: 60,
            requests_per_day: 10000,
            concurrent_invocations: 10,
        }
    }
}

/// Authenticated user context
#[derive(Debug, Clone)]
pub struct AuthContext {
    pub user_id: String,
    pub permissions: Vec<Permission>,
    pub rate_limit: RateLimit,
}

impl AuthContext {
    pub fn has_permission(&self, permission: &Permission) -> bool {
        self.permissions.contains(&Permission::Admin) || self.permissions.contains(permission)
    }
}

// ============================================================================
// Usage & Metrics Types
// ============================================================================

/// Usage record for billing
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UsageRecord {
    pub user_id: String,
    pub function_id: Option<String>,
    pub operation: UsageOperation,
    pub timestamp: DateTime<Utc>,
    pub duration_ms: u64,
    pub memory_mb: u32,
    pub request_size_bytes: u64,
    pub response_size_bytes: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum UsageOperation {
    FunctionInvoke,
    VectorUpsert,
    VectorQuery,
    VectorDelete,
    AiEmbedding,
    AiTextGeneration,
}

// ============================================================================
// Health & Status Types
// ============================================================================

/// Health check response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthResponse {
    pub status: &'static str,
    pub version: &'static str,
    pub region: String,
    pub timestamp: DateTime<Utc>,
}
