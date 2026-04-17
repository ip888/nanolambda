//! Error types for the NanoVM Virtual Machine Monitor.
//!
//! This module provides comprehensive error handling with actionable error messages
//! and proper error categorization for production debugging.
//!
//! # Error Categories
//!
//! | Category | Description | Typical Cause |
//! |----------|-------------|---------------|
//! | KVM | Hypervisor errors | Missing KVM, permissions |
//! | Memory | Memory allocation/mapping | OOM, invalid regions |
//! | WASM | WebAssembly execution | Invalid bytecode, traps |
//! | Config | Configuration errors | Invalid parameters |
//! | Resource | Resource exhaustion | Too many VMs, no capacity |
//!
//! # Error Handling Best Practices
//!
//! ```rust
//! use nanolambda_vmm::{VmmError, VmmResult};
//!
//! fn handle_error(result: VmmResult<()>) {
//!     match result {
//!         Ok(()) => println!("Success"),
//!         Err(VmmError::KvmNotAvailable) => {
//!             eprintln!("KVM not available - check that virtualization is enabled in BIOS");
//!             // Suggest fallback or graceful degradation
//!         }
//!         Err(VmmError::ResourceExhausted(msg)) => {
//!             eprintln!("Resource limit reached: {}", msg);
//!             // Trigger scale-up or queue request
//!         }
//!         Err(e) => {
//!             eprintln!("Unexpected error: {}", e);
//!             // Log and alert
//!         }
//!     }
//! }
//! ```

use std::fmt;
use thiserror::Error;

/// Type alias for Results with [`VmmError`].
///
/// Use this as the return type for all VMM operations that can fail.
///
/// # Example
///
/// ```rust
/// use nanolambda_vmm::{VmmResult, VmmError};
///
/// fn create_vm() -> VmmResult<()> {
///     // ... implementation
///     Ok(())
/// }
/// ```
pub type VmmResult<T> = Result<T, VmmError>;

/// Comprehensive error type for all VMM operations.
///
/// Each variant includes actionable information for debugging and user feedback.
/// Errors are designed to be:
/// - **Specific**: Clear indication of what went wrong
/// - **Actionable**: Guidance on how to fix the issue
/// - **Debuggable**: Sufficient context for post-mortem analysis
///
/// # Stability
///
/// New error variants may be added in minor versions. Always use a catch-all
/// pattern when matching:
///
/// ```rust
/// # use nanolambda_vmm::VmmError;
/// # fn handle(e: VmmError) {
/// match e {
///     VmmError::KvmNotAvailable => { /* handle */ }
///     other => { /* catch-all for new variants */ }
/// }
/// # }
/// ```
#[derive(Error, Debug)]
pub enum VmmError {
    // ========================================================================
    // KVM/Hypervisor Errors
    // ========================================================================
    /// KVM kernel module is not loaded or not available.
    ///
    /// # Resolution
    /// 1. Verify virtualization is enabled in BIOS/UEFI
    /// 2. Load the KVM module: `modprobe kvm_intel` or `modprobe kvm_amd`
    /// 3. Verify /dev/kvm exists
    #[error(
        "KVM is not available on this system. Ensure virtualization is enabled in BIOS and load the kvm kernel module (modprobe kvm_intel or kvm_amd)."
    )]
    KvmNotAvailable,

    /// Permission denied when accessing /dev/kvm.
    ///
    /// # Resolution
    /// - Add user to 'kvm' group: `sudo usermod -aG kvm $USER`
    /// - Log out and back in for group changes to take effect
    #[error(
        "Permission denied accessing /dev/kvm. Add user to 'kvm' group: sudo usermod -aG kvm $USER"
    )]
    KvmPermissionDenied,

    /// A KVM ioctl system call failed.
    ///
    /// The contained string describes which ioctl failed.
    #[error("KVM ioctl failed: {0}")]
    KvmIoctlFailed(&'static str),

    /// KVM API version is incompatible.
    ///
    /// NanoVM requires KVM API version 12.
    #[error("Unsupported KVM API version: {0}. Expected version 12. Update your kernel.")]
    KvmUnsupportedVersion(i32),

    /// A required KVM capability is not supported.
    #[error("KVM missing required capability: {0}. This feature requires a newer kernel.")]
    KvmMissingCapability(&'static str),

    /// Generic KVM error with context message.
    #[error("KVM error: {0}")]
    KvmError(String),

    /// I/O error during KVM operations.
    #[error("KVM I/O error: {0}")]
    KvmIoError(#[from] std::io::Error),

    // ========================================================================
    // Memory Errors
    // ========================================================================
    /// Memory allocation failed.
    ///
    /// # Causes
    /// - System out of memory
    /// - Memory limit reached
    /// - mmap failed
    #[error("Memory allocation failed: {0}")]
    MemoryAllocationFailed(String),

    /// Memory region configuration is invalid.
    #[error("Invalid memory region: {0}")]
    InvalidMemoryRegion(String),

    /// Memory regions overlap, which is not allowed.
    #[error("Memory region overlap detected. Each memory region must have a unique address range.")]
    MemoryRegionOverlap,

    /// Generic memory operation error.
    #[error("Memory error: {0}")]
    MemoryError(String),

    // ========================================================================
    // vCPU Errors
    // ========================================================================
    /// Failed to create a virtual CPU.
    #[error("vCPU creation failed: {0}")]
    VCpuCreationFailed(String),

    /// vCPU execution failed.
    #[error("vCPU run failed: {0}")]
    VCpuRunFailed(String),

    // ========================================================================
    // VM Lifecycle Errors
    // ========================================================================
    /// VM configuration is invalid.
    #[error("VM configuration invalid: {0}")]
    InvalidConfig(String),

    /// Attempted to start an already running VM.
    #[error("VM already running. Stop the VM before starting again.")]
    VmAlreadyRunning,

    /// Operation requires a running VM but VM is stopped.
    #[error("VM not running. Start the VM first.")]
    VmNotRunning,

    /// Boot protocol error (kernel loading, etc.).
    #[error("Boot protocol error: {0}")]
    BootError(String),

    /// Virtual device error.
    #[error("Device error: {0}")]
    DeviceError(String),

    // ========================================================================
    // Snapshot/Clone Errors
    // ========================================================================
    /// Snapshot operation failed.
    #[error("Snapshot error: {0}")]
    SnapshotError(String),

    /// Clone pool operation failed.
    #[error("Clone pool error: {0}")]
    CloneError(String),

    // ========================================================================
    // WASM Errors
    // ========================================================================
    /// WASM execution error (compilation, runtime, etc.).
    #[error("WASM error: {0}")]
    WasmError(String),

    /// WASM sandbox needs promotion to full VM.
    ///
    /// This is not necessarily an error - it indicates the function
    /// requires capabilities beyond the WASM sandbox.
    #[error("WASM promotion required: {0}")]
    WasmPromotionRequired(String),

    // ========================================================================
    // Configuration Errors
    // ========================================================================
    /// Configuration parameter is invalid.
    #[error("Configuration error: {0}")]
    ConfigError(String),

    /// Requested feature is not supported on this system.
    #[error("Feature not supported: {0}")]
    FeatureNotSupported(String),

    // ========================================================================
    // Resource Errors
    // ========================================================================
    /// Resource limit has been reached.
    ///
    /// # Causes
    /// - Maximum concurrent VMs reached
    /// - Memory limit reached
    /// - File descriptor limit reached
    #[error("Resource exhausted: {0}")]
    ResourceExhausted(String),

    /// Internal synchronization error (lock poisoned).
    ///
    /// This indicates a panic occurred while holding a lock.
    /// The system may be in an inconsistent state.
    #[error("Internal lock poisoned - a thread panicked while holding a lock")]
    LockPoisoned,
}

impl VmmError {
    /// Check if this error is recoverable.
    ///
    /// Recoverable errors can potentially be retried or worked around.
    /// Non-recoverable errors indicate fundamental system issues.
    ///
    /// # Example
    ///
    /// ```rust
    /// # use nanolambda_vmm::VmmError;
    /// # fn should_retry(e: &VmmError) -> bool {
    /// if e.is_recoverable() {
    ///     // Retry with backoff
    ///     true
    /// } else {
    ///     // Log and alert
    ///     false
    /// }
    /// # }
    /// ```
    pub fn is_recoverable(&self) -> bool {
        matches!(
            self,
            VmmError::ResourceExhausted(_)
                | VmmError::WasmPromotionRequired(_)
                | VmmError::MemoryAllocationFailed(_)
        )
    }

    /// Check if this error is a user/configuration error.
    ///
    /// User errors are typically caused by invalid input or configuration,
    /// not system failures.
    pub fn is_user_error(&self) -> bool {
        matches!(
            self,
            VmmError::InvalidConfig(_)
                | VmmError::ConfigError(_)
                | VmmError::InvalidMemoryRegion(_)
                | VmmError::WasmError(_)
        )
    }

    /// Get a short error code for metrics/logging.
    ///
    /// # Example
    ///
    /// ```rust
    /// # use nanolambda_vmm::VmmError;
    /// let error = VmmError::KvmNotAvailable;
    /// assert_eq!(error.error_code(), "KVM_NOT_AVAILABLE");
    /// ```
    pub fn error_code(&self) -> &'static str {
        match self {
            VmmError::KvmNotAvailable => "KVM_NOT_AVAILABLE",
            VmmError::KvmPermissionDenied => "KVM_PERMISSION_DENIED",
            VmmError::KvmIoctlFailed(_) => "KVM_IOCTL_FAILED",
            VmmError::KvmUnsupportedVersion(_) => "KVM_UNSUPPORTED_VERSION",
            VmmError::KvmMissingCapability(_) => "KVM_MISSING_CAPABILITY",
            VmmError::KvmError(_) => "KVM_ERROR",
            VmmError::KvmIoError(_) => "KVM_IO_ERROR",
            VmmError::MemoryAllocationFailed(_) => "MEMORY_ALLOCATION_FAILED",
            VmmError::InvalidMemoryRegion(_) => "INVALID_MEMORY_REGION",
            VmmError::MemoryRegionOverlap => "MEMORY_REGION_OVERLAP",
            VmmError::MemoryError(_) => "MEMORY_ERROR",
            VmmError::VCpuCreationFailed(_) => "VCPU_CREATION_FAILED",
            VmmError::VCpuRunFailed(_) => "VCPU_RUN_FAILED",
            VmmError::InvalidConfig(_) => "INVALID_CONFIG",
            VmmError::VmAlreadyRunning => "VM_ALREADY_RUNNING",
            VmmError::VmNotRunning => "VM_NOT_RUNNING",
            VmmError::BootError(_) => "BOOT_ERROR",
            VmmError::DeviceError(_) => "DEVICE_ERROR",
            VmmError::SnapshotError(_) => "SNAPSHOT_ERROR",
            VmmError::CloneError(_) => "CLONE_ERROR",
            VmmError::WasmError(_) => "WASM_ERROR",
            VmmError::WasmPromotionRequired(_) => "WASM_PROMOTION_REQUIRED",
            VmmError::ConfigError(_) => "CONFIG_ERROR",
            VmmError::FeatureNotSupported(_) => "FEATURE_NOT_SUPPORTED",
            VmmError::ResourceExhausted(_) => "RESOURCE_EXHAUSTED",
            VmmError::LockPoisoned => "LOCK_POISONED",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_display() {
        let error = VmmError::KvmNotAvailable;
        let display = format!("{}", error);
        assert!(display.contains("KVM"));
        assert!(display.contains("modprobe"));
    }

    #[test]
    fn test_error_codes() {
        assert_eq!(VmmError::KvmNotAvailable.error_code(), "KVM_NOT_AVAILABLE");
        assert_eq!(
            VmmError::ResourceExhausted("test".to_string()).error_code(),
            "RESOURCE_EXHAUSTED"
        );
    }

    #[test]
    fn test_recoverable_errors() {
        assert!(VmmError::ResourceExhausted("test".to_string()).is_recoverable());
        assert!(VmmError::WasmPromotionRequired("test".to_string()).is_recoverable());
        assert!(!VmmError::KvmNotAvailable.is_recoverable());
        assert!(!VmmError::LockPoisoned.is_recoverable());
    }

    #[test]
    fn test_user_errors() {
        assert!(VmmError::InvalidConfig("test".to_string()).is_user_error());
        assert!(VmmError::WasmError("test".to_string()).is_user_error());
        assert!(!VmmError::KvmNotAvailable.is_user_error());
    }

    #[test]
    fn test_io_error_conversion() {
        let io_error = std::io::Error::new(std::io::ErrorKind::NotFound, "file not found");
        let vmm_error: VmmError = io_error.into();
        assert!(matches!(vmm_error, VmmError::KvmIoError(_)));
    }
}
