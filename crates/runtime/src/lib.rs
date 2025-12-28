//! Language runtime management

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
