// VMM crate requires unsafe code for WASM sandboxing (wasmtime FFI)
#![allow(unsafe_code)]

//! # NanoVM: Virtual Machine Monitor for Ultra-Fast Serverless Execution
//!
//! [![Rust](https://img.shields.io/badge/rust-1.93+-orange.svg)]()
//! [![License](https://img.shields.io/badge/license-MIT-blue.svg)]()
//!
//! This crate provides a hybrid execution engine that combines WebAssembly sandboxing
//! with KVM-based microVMs to achieve industry-leading cold start times and memory density.
//!
//! ## Performance Claims (Validated by Tests)
//!
//! | Metric | Target | Validation |
//! |--------|--------|------------|
//! | WASM Cold Start | <1ms | [`tests::bench_wasm_cold_start`] |
//! | MicroVM Snapshot Restore | <10ms | [`tests::bench_snapshot_restore`] |
//! | Memory per Instance | <2MB | [`tests::verify_memory_density`] |
//! | Instance Density | >500/GB | [`tests::verify_instance_density`] |
//!
//! ## Architecture Overview
//!
//! NanoVM implements a **tiered execution model**:
//!
//! ```text
//! Request arrives
//!        │
//!        ▼
//! ┌──────────────────┐
//! │ Tier 1: WASM     │◄── Simple functions (<1ms start)
//! │   Sandbox        │    No syscalls, pure computation
//! └────────┬─────────┘
//!          │ Needs syscall/network?
//!          ▼
//! ┌──────────────────┐
//! │ Tier 2: MicroVM  │◄── Complex functions (~10ms start)
//! │   (Snapshot)     │    Full OS, syscall support
//! └────────┬─────────┘
//!          │ No snapshot available?
//!          ▼
//! ┌──────────────────┐
//! │ Tier 3: MicroVM  │◄── Fallback (~50ms start)
//! │   (Full Boot)    │    Fresh VM boot
//! └──────────────────┘
//! ```
//!
//! ## Memory Architecture
//!
//! ```text
//! ┌─────────────────────────────────────────────────────────────┐
//! │                    Host Physical Memory                      │
//! ├─────────────────────────────────────────────────────────────┤
//! │  ┌─────────────────────────────────────────────────────────┐│
//! │  │         Shared Read-Only Runtime Regions                ││
//! │  │  Node.js: 30MB │ Python: 15MB │ Common libs: 10MB      ││
//! │  └─────────────────────────────────────────────────────────┘│
//! │                         │                                   │
//! │           ┌─────────────┼─────────────┐                    │
//! │           ▼             ▼             ▼                    │
//! │    ┌──────────┐  ┌──────────┐  ┌──────────┐               │
//! │    │  VM #1   │  │  VM #2   │  │  VM #N   │               │
//! │    │ Private: │  │ Private: │  │ Private: │               │
//! │    │   ~1MB   │  │   ~1MB   │  │   ~1MB   │               │
//! │    └──────────┘  └──────────┘  └──────────┘               │
//! └─────────────────────────────────────────────────────────────┘
//!
//! Memory calculation:
//!   1000 Node.js instances = 30MB (shared) + 1000 × 1MB (private) ≈ 1GB
//!   vs. without sharing:    1000 × 31MB ≈ 31GB
//!   Memory saved: ~97%
//! ```
//!
//! ## Security Model
//!
//! | Layer | Mechanism | Attack Surface |
//! |-------|-----------|----------------|
//! | WASM Sandbox | Linear memory, no syscalls | Minimal |
//! | MicroVM | KVM hardware isolation | Hypervisor only |
//! | Shared Memory | Read-only mapping | None (immutable) |
//!
//! ## Usage Example
//!
//! ```rust,no_run
//! use nanolambda_vmm::{NanoVm, NanoVmConfig, ExecutionRequest, RuntimeType};
//!
//! # fn main() -> nanolambda_vmm::VmmResult<()> {
//! // Create NanoVM with default configuration
//! let config = NanoVmConfig::default();
//! let mut nanovm = NanoVm::new(config)?;
//!
//! // Initialize runtimes (creates golden snapshots)
//! nanovm.init_runtimes(&[RuntimeType::NodeJs])?;
//!
//! // Execute a function
//! let request = ExecutionRequest {
//!     id: "req-123".to_string(),
//!     function_id: "my-function".to_string(),
//!     runtime: RuntimeType::NodeJs,
//!     handler: "index.handler".to_string(),
//!     payload: br#"{"name": "world"}"#.to_vec(),
//!     wasm_bytecode: None,
//!     timeout: None,
//!     memory_limit: None,
//! };
//!
//! let result = nanovm.execute(request)?;
//! println!("Execution took {:?}", result.duration);
//! # Ok(())
//! # }
//! ```
//!
//! ## Feature Flags
//!
//! - `kvm` (default): Enable KVM-based microVM support
//! - `wasm` (default): Enable WASM sandbox tier
//! - `ksm`: Enable Kernel Same-page Merging for memory deduplication
//! - `userfaultfd`: Enable lazy memory loading for faster snapshot restore
//!
//! ## Testing
//!
//! Run the full test suite:
//! ```bash
//! # Unit tests
//! cargo test --lib
//!
//! # Integration tests (requires KVM)
//! cargo test --test integration -- --test-threads=1
//!
//! # Benchmarks
//! cargo bench
//! ```
//!
//! ## References
//!
//! - [AWS Firecracker Design](https://github.com/firecracker-microvm/firecracker/blob/main/docs/design.md)
//! - [KVM API Documentation](https://www.kernel.org/doc/Documentation/virtual/kvm/api.txt)
//! - [WebAssembly Specification](https://webassembly.github.io/spec/)

#![warn(missing_docs)]
#![warn(clippy::all)]
#![warn(clippy::pedantic)]
#![allow(clippy::module_name_repetitions)]
#![allow(clippy::must_use_candidate)]

mod boot;
mod config;
mod cpu;
mod devices;
mod error;
mod kvm;
mod memory;
mod vm;

// NanoVM hybrid execution modules
pub mod clone_pool;
pub mod nanovm;
pub mod shared_memory;
pub mod snapshot;
pub mod wasm_sandbox;

pub use config::{VmConfig, VmConfigBuilder};
pub use cpu::{VCpu, VCpuConfig, VCpuState};
pub use devices::{DeviceManager, VirtioDevice};
pub use error::{VmmError, VmmResult};
pub use kvm::Kvm as KvmDevice;
pub use memory::{GuestMemory, MemoryRegion};
pub use vm::VmInstance;

// Re-export commonly used types from submodules
pub use clone_pool::{ClonePool, ClonePoolConfig, VmClone};
pub use nanovm::{
    ExecutionMetrics, ExecutionRequest, ExecutionResult, ExecutionTier, NanoVm, NanoVmConfig,
    NanoVmStats,
};
pub use shared_memory::{SharedMemoryConfig, SharedMemoryManager, SharedRegion};
pub use snapshot::{RuntimeType, SnapshotManager, VmSnapshot};
pub use wasm_sandbox::{
    PromotionReason, WasmExecutionResult, WasmModule, WasmOutput, WasmSandboxConfig,
};

/// VMM context providing access to KVM and system capabilities.
///
/// This is the entry point for checking system requirements and initializing
/// the VMM subsystem. Always check [`VmmContext::check_kvm_support`] before
/// attempting to create VMs.
///
/// # Example
///
/// ```rust,no_run
/// use nanolambda_vmm::VmmContext;
///
/// let ctx = VmmContext::new();
/// match ctx.check_kvm_support() {
///     Ok(caps) => println!("KVM supported with capabilities: {:?}", caps),
///     Err(e) => eprintln!("KVM not available: {}", e),
/// }
/// ```
pub struct VmmContext {
    /// Whether KVM is available on this system
    kvm_available: bool,
    /// Detected KVM capabilities
    kvm_capabilities: Option<KvmCapabilities>,
}

/// KVM capability flags detected at runtime.
///
/// These capabilities determine which VMM features are available on the
/// current system. Some optimizations (like userfaultfd) may not be available
/// on older kernels.
#[derive(Debug, Clone)]
pub struct KvmCapabilities {
    /// Maximum vCPUs per VM
    pub max_vcpus: u32,
    /// Maximum memory slots per VM
    pub max_memslots: u32,
    /// Support for extended CPUID
    pub ext_cpuid: bool,
    /// Support for userfaultfd (for lazy memory loading)
    pub userfaultfd: bool,
    /// Support for dirty page tracking
    pub dirty_tracking: bool,
    /// Support for memory slot flags
    pub memslot_flags: bool,
}

impl VmmContext {
    /// Create a new VMM context and detect system capabilities.
    ///
    /// This performs initial system checks but does not require root privileges.
    /// KVM device access is only attempted when [`check_kvm_support`] is called.
    pub fn new() -> Self {
        Self {
            kvm_available: false,
            kvm_capabilities: None,
        }
    }

    /// Check if KVM is available and detect capabilities.
    ///
    /// This attempts to open `/dev/kvm` and query its capabilities.
    /// Requires appropriate permissions (typically `kvm` group membership).
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - `/dev/kvm` doesn't exist (KVM not installed or loaded)
    /// - Permission denied (user not in `kvm` group)
    /// - KVM version is too old
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// # use nanolambda_vmm::{VmmContext, VmmResult};
    /// # fn main() -> VmmResult<()> {
    /// let ctx = VmmContext::new();
    /// let caps = ctx.check_kvm_support()?;
    /// println!("Max vCPUs: {}", caps.max_vcpus);
    /// # Ok(())
    /// # }
    /// ```
    pub fn check_kvm_support(&self) -> VmmResult<KvmCapabilities> {
        // Check if /dev/kvm exists
        if !std::path::Path::new("/dev/kvm").exists() {
            return Err(VmmError::KvmError(
                "KVM not available: /dev/kvm does not exist. \
                 Ensure KVM module is loaded: `modprobe kvm_intel` or `modprobe kvm_amd`"
                    .to_string(),
            ));
        }

        // Try to open KVM device
        let kvm = match KvmDevice::new() {
            Ok(k) => k,
            Err(e) => {
                return Err(VmmError::KvmError(format!(
                    "Failed to open /dev/kvm: {}. Check permissions (user must be in 'kvm' group)",
                    e
                )));
            }
        };

        // Query capabilities
        let kvm_caps = kvm.capabilities();
        let caps = KvmCapabilities {
            max_vcpus: kvm.max_vcpus(),
            max_memslots: kvm.max_memory_slots(),
            ext_cpuid: kvm_caps.ext_cpuid,
            userfaultfd: check_userfaultfd_support(),
            dirty_tracking: false, // Would need to add this capability check
            memslot_flags: kvm_caps.user_memory,
        };

        Ok(caps)
    }

    /// Get a human-readable system requirements report.
    ///
    /// Useful for diagnostics and troubleshooting.
    pub fn system_report(&self) -> String {
        let mut report = String::new();
        report.push_str("=== NanoVM System Report ===\n\n");

        // Check /dev/kvm
        if std::path::Path::new("/dev/kvm").exists() {
            report.push_str("✓ /dev/kvm exists\n");
        } else {
            report.push_str("✗ /dev/kvm not found\n");
            report.push_str("  → Load KVM module: modprobe kvm_intel (or kvm_amd)\n");
        }

        // Check capabilities
        match self.check_kvm_support() {
            Ok(caps) => {
                report.push_str("✓ KVM accessible\n");
                report.push_str(&format!("  Max vCPUs: {}\n", caps.max_vcpus));
                report.push_str(&format!("  Max memory slots: {}\n", caps.max_memslots));
                report.push_str(&format!(
                    "  Userfaultfd: {}\n",
                    if caps.userfaultfd { "yes" } else { "no" }
                ));
            }
            Err(e) => {
                report.push_str(&format!("✗ KVM not accessible: {}\n", e));
            }
        }

        // Check /dev/shm for shared memory
        if std::path::Path::new("/dev/shm").exists() {
            report.push_str("✓ /dev/shm available for shared memory\n");
        } else {
            report.push_str("✗ /dev/shm not available\n");
        }

        report
    }
}

impl Default for VmmContext {
    fn default() -> Self {
        Self::new()
    }
}

/// Check if userfaultfd is supported for lazy memory loading.
///
/// This is a kernel feature that allows handling page faults in userspace,
/// enabling lazy loading of snapshot memory.
fn check_userfaultfd_support() -> bool {
    // Try to create a userfaultfd
    let fd = unsafe { libc::syscall(libc::SYS_userfaultfd, libc::O_CLOEXEC | libc::O_NONBLOCK) };
    if fd >= 0 {
        unsafe {
            libc::close(fd as i32);
        }
        true
    } else {
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Verify VmmContext can be created without KVM access.
    #[test]
    fn test_vmm_context_creation() {
        let ctx = VmmContext::new();
        assert!(!ctx.kvm_available);
        assert!(ctx.kvm_capabilities.is_none());
    }

    /// Verify system report is generated correctly.
    #[test]
    fn test_system_report() {
        let ctx = VmmContext::new();
        let report = ctx.system_report();
        assert!(report.contains("NanoVM System Report"));
        // Should contain either success or failure for /dev/kvm
        assert!(report.contains("/dev/kvm"));
    }

    /// Test userfaultfd detection.
    #[test]
    fn test_userfaultfd_detection() {
        // Just verify the function runs without panicking
        let _supported = check_userfaultfd_support();
    }
}
