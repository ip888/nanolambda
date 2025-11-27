//! MicroVM Manager - Core virtualization engine (POC Version)
//!
//! This crate provides a minimal viable implementation of a microVM
//! for running isolated Python functions.

// POC implementation modules
pub mod vm_poc;
pub mod memory_poc;
pub mod python_poc;
pub mod vcpu;
pub mod devices;

// Re-exports for POC
pub use vm_poc::{Vm, VmConfig};
pub use memory_poc::{MemoryManager, MemoryConfig};
pub use python_poc::{PythonRuntime, PythonFunctionConfig};

use thiserror::Error;

#[derive(Error, Debug)]
pub enum VmmError {
    #[error("KVM error: {0}")]
    Kvm(kvm_ioctls::Error),
    
    #[error("Memory error: {0}")]
    Memory(vm_memory::GuestMemoryError),

    #[error("Memory mapping error: {0}")]
    MmapRegion(vm_memory::mmap::MmapRegionError),

    #[error("IO error: {0}")]
    Io(std::io::Error),

    #[error("Linux loader error: {0}")]
    LinuxLoader(linux_loader::loader::Error),

    #[error("Invalid configuration: {0}")]
    InvalidConfig(String),
    
    #[error("Invalid state: {0}")]
    InvalidState(String),
    
    #[error("Runtime error: {0}")]
    RuntimeError(String),
}

impl From<kvm_ioctls::Error> for VmmError {
    fn from(err: kvm_ioctls::Error) -> Self {
        VmmError::Kvm(err)
    }
}

impl From<vm_memory::GuestMemoryError> for VmmError {
    fn from(err: vm_memory::GuestMemoryError) -> Self {
        VmmError::Memory(err)
    }
}

impl From<vm_memory::mmap::MmapRegionError> for VmmError {
    fn from(err: vm_memory::mmap::MmapRegionError) -> Self {
        VmmError::MmapRegion(err)
    }
}

impl From<std::io::Error> for VmmError {
    fn from(err: std::io::Error) -> Self {
        VmmError::Io(err)
    }
}

impl From<linux_loader::loader::Error> for VmmError {
    fn from(err: linux_loader::loader::Error) -> Self {
        VmmError::LinuxLoader(err)
    }
}

pub type Result<T> = std::result::Result<T, VmmError>;
