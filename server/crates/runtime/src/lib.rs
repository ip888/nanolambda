//! Language runtime management for NanoLambda serverless platform.
//!
//! This crate provides multi-language function execution with resource isolation,
//! warm start optimization, and comprehensive metrics collection.
//!
//! # Architecture
//!
//! The runtime system is built around these core concepts:
//!
//! - **[`Runtime`] trait**: Abstract interface for language-specific executors
//! - **Process pools**: Pre-warmed interpreter processes for fast cold starts
//! - **Resource limits**: Memory, CPU time, and network isolation via cgroups
//!
//! # Supported Languages
//!
//! | Language | Module | Cold Start | Warm Start |
//! |----------|--------|------------|------------|
//! | Python 3.x | [`python`] | ~200ms | ~5ms |
//! | Node.js | [`nodejs`] | ~150ms | ~3ms |
//! | Java | [`java`] | ~500ms | ~10ms |
//!
//! # Execution Flow
//!
//! ```text
//! Request → Pool Manager → Get/Create Process → Execute Handler → Collect Metrics → Return Result
//!            │                    │                    │
//!            │                    │                    └─ Timeout/memory enforcement
//!            │                    └─ Process warm-up or cold start
//!            └─ Select by language, check pool health
//! ```
//!
//! # Example
//!
//! ```rust,ignore
//! use nanolambda_runtime::{PythonExecutor, Runtime, FunctionConfig};
//!
//! let executor = PythonExecutor::new()?;
//! let config = FunctionConfig {
//!     handler: "main.handler".to_string(),
//!     memory_mb: 128,
//!     timeout_ms: 30000,
//!     // ...
//! };
//!
//! let result = executor.invoke(&config, payload).await?;
//! ```
//!
//! # Security
//!
//! Each function execution runs in isolation with:
//! - Separate process namespace
//! - Memory limits enforced via cgroups (Linux) or process limits
//! - Network restrictions (configurable)
//! - Read-only filesystem mounts for code

pub mod executor;
pub mod java;
pub mod metrics;
pub mod nodejs;
pub mod pool;
pub mod python;
pub mod runtime_trait;
pub mod types;

// Re-export main types
pub use executor::{
    ExecutionMetrics, ExecutionResult, ExecutorError, FunctionConfig, PythonExecutor,
};

pub use pool::{PoolError, ProcessPool, ProcessStats};

pub use metrics::{MetricsError, ProcessMetrics};

pub use types::{
    GenericFunctionConfig, InvocationResult, Language, RuntimeCapabilities, RuntimeInfo,
};

pub use runtime_trait::Runtime;

pub use nodejs::{NodeError, NodeJSExecutor, NodeProcess, NodeVersion};

pub use java::{JavaError, JavaExecutor};
