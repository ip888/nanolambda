//! NanoLambda - Self-hosted serverless platform
//!
//! AWS Lambda-compatible serverless with microVM isolation.

pub use nanolambda_vmm as vmm;
pub use nanolambda_api as api;
pub use nanolambda_runtime as runtime;
pub use nanolambda_scheduler as scheduler;
pub use nanolambda_storage as storage;

/// Project version
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// Project name
pub const NAME: &str = env!("CARGO_PKG_NAME");

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_version() {
        assert!(!VERSION.is_empty());
    }
}
