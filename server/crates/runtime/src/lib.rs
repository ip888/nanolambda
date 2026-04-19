//! Python function runtime for the NanoLambda sandbox.
//!
//! Single-language executor (Python 3.11/3.12) with a pre-warmed process pool
//! and per-invocation resource limits. Node.js and Java support was removed
//! in the 2026-04 niche refocus onto AI-agent code-execution sandboxing;
//! they will be reintroduced only if customer demand justifies it.

pub mod executor;
pub mod metrics;
pub mod pool;

// Re-export main types
pub use executor::{
    ExecutionMetrics, ExecutionResult, ExecutorError, FunctionConfig, PythonExecutor,
};

pub use pool::{PoolError, ProcessPool, ProcessStats};

pub use metrics::{MetricsError, ProcessMetrics};
