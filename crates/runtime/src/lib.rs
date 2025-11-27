//! Language runtime management

pub mod executor;
pub mod pool;
pub mod python;
pub mod nodejs;
pub mod java;
pub mod metrics;
pub mod types;
pub mod runtime_trait;

// Re-export main types
pub use executor::{
    PythonExecutor,
    FunctionConfig,
    ExecutionResult,
    ExecutionMetrics,
    ExecutorError,
};

pub use pool::{
    ProcessPool,
    ProcessStats,
    PoolError,
};

pub use metrics::{
    ProcessMetrics,
    MetricsError,
};

pub use types::{
    Language,
    RuntimeInfo,
    RuntimeCapabilities,
    InvocationResult,
    GenericFunctionConfig,
};

pub use runtime_trait::{
    Runtime,
};

pub use nodejs::{
    NodeJSExecutor,
    NodeProcess,
    NodeError,
    NodeVersion,
};
