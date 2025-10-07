//! MicroVM Manager - Core virtualization engine
//!
//! This crate provides the core microVM management functionality using KVM.
//! It handles VM lifecycle, memory management, and device emulation.

pub mod vm;
pub mod vcpu;
pub mod memory;
pub mod devices;
pub mod snapshot;

pub use vm::{Vm, VmConfig, VmState};
pub use vcpu::Vcpu;
pub use memory::GuestMemory;
pub use snapshot::{Snapshot, SnapshotManager};

use thiserror::Error;

#[derive(Error, Debug)]
pub enum VmmError {
    #[error("KVM error: {0}")]
    Kvm(#[from] kvm_ioctls::Error),
    
    #[error("Memory error: {0}")]
    Memory(#[from] vm_memory::GuestMemoryError),
    
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    
    #[error("VM not found: {0}")]
    VmNotFound(String),
    
    #[error("Invalid configuration: {0}")]
    InvalidConfig(String),
    
    #[error("Timeout: {0}")]
    Timeout(String),
}

pub type Result<T> = std::result::Result<T, VmmError>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_placeholder() {
        // Placeholder test
        assert!(true);
    }
}
