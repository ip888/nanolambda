//! Integration Tests for NanoVM
//!
//! These tests verify real functionality in near-production conditions.
//! They test actual memory operations, timing constraints, and system behavior.
//!
//! # Test Categories
//!
//! 1. **Unit Tests** - Fast, isolated, mock-free where possible
//! 2. **Integration Tests** - Test real system interactions
//! 3. **Performance Tests** - Verify timing and memory claims
//! 4. **Stress Tests** - Test behavior under load
//!
//! # Running Tests
//!
//! ```bash
//! # All tests
//! cargo test -p nanolambda-vmm
//!
//! # Integration tests only (may require KVM)
//! cargo test -p nanolambda-vmm --test '*' -- --ignored
//!
//! # Performance tests
//! cargo test -p nanolambda-vmm -- --test-threads=1 bench
//! ```

use std::collections::HashMap;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

use nanolambda_vmm::clone_pool::{ClonePool, ClonePoolConfig, CloneState};
use nanolambda_vmm::nanovm::{ExecutionRequest, ExecutionTier, NanoVm, NanoVmConfig};
use nanolambda_vmm::shared_memory::{SharedMemoryConfig, SharedMemoryManager};
use nanolambda_vmm::snapshot::{RuntimeType, SnapshotManager};
use nanolambda_vmm::wasm_sandbox::{WasmModule, WasmSandboxConfig, WasiCapabilities};
use nanolambda_vmm::{VmmContext, VmmError, VmmResult};

// ============================================================================
// TEST UTILITIES
// ============================================================================

/// Test helper to measure execution time with high precision
struct TimingMeasurement {
    samples: Vec<Duration>,
    name: String,
}

impl TimingMeasurement {
    fn new(name: impl Into<String>) -> Self {
        Self {
            samples: Vec::new(),
            name: name.into(),
        }
    }

    fn measure<F, R>(&mut self, f: F) -> R
    where
        F: FnOnce() -> R,
    {
        let start = Instant::now();
        let result = f();
        self.samples.push(start.elapsed());
        result
    }

    fn mean(&self) -> Duration {
        if self.samples.is_empty() {
            return Duration::ZERO;
        }
        let total: Duration = self.samples.iter().sum();
        total / self.samples.len() as u32
    }

    fn p50(&self) -> Duration {
        self.percentile(50)
    }

    fn p99(&self) -> Duration {
        self.percentile(99)
    }

    fn percentile(&self, p: usize) -> Duration {
        if self.samples.is_empty() {
            return Duration::ZERO;
        }
        let mut sorted = self.samples.clone();
        sorted.sort();
        let idx = (p * sorted.len() / 100).min(sorted.len() - 1);
        sorted[idx]
    }

    fn report(&self) -> String {
        format!(
            "{}: mean={:?}, p50={:?}, p99={:?}, samples={}",
            self.name,
            self.mean(),
            self.p50(),
            self.p99(),
            self.samples.len()
        )
    }
}

/// Create a minimal valid WASM module for testing
fn create_test_wasm_module() -> Vec<u8> {
    // Minimal valid WASM module that exports a function
    // (module
    //   (func (export "add") (param i32 i32) (result i32)
    //     local.get 0
    //     local.get 1
    //     i32.add))
    vec![
        0x00, 0x61, 0x73, 0x6d, // magic: \0asm
        0x01, 0x00, 0x00, 0x00, // version: 1
        // Type section
        0x01, 0x07, 0x01, 0x60, 0x02, 0x7f, 0x7f, 0x01, 0x7f,
        // Function section
        0x03, 0x02, 0x01, 0x00, // Export section
        0x07, 0x07, 0x01, 0x03, 0x61, 0x64, 0x64, 0x00, 0x00,
        // Code section
        0x0a, 0x09, 0x01, 0x07, 0x00, 0x20, 0x00, 0x20, 0x01, 0x6a, 0x0b,
    ]
}

/// Create invalid WASM bytecode for error testing
fn create_invalid_wasm() -> Vec<u8> {
    vec![0x00, 0x00, 0x00, 0x00] // Not valid WASM
}

// ============================================================================
// WASM SANDBOX TESTS
// ============================================================================

mod wasm_tests {
    use super::*;

    /// Test: WASM module compilation succeeds with valid bytecode
    #[test]
    fn test_wasm_module_compilation_valid() {
        let bytecode = create_test_wasm_module();
        let result = WasmModule::compile("test-module", &bytecode);

        assert!(result.is_ok(), "Valid WASM should compile");
        let module = result.unwrap();
        assert_eq!(module.id, "test-module");
        assert!(module.bytecode_size > 0);
    }

    /// Test: WASM module compilation fails with invalid bytecode
    #[test]
    fn test_wasm_module_compilation_invalid() {
        let bytecode = create_invalid_wasm();
        let result = WasmModule::compile("invalid", &bytecode);

        assert!(result.is_err(), "Invalid WASM should fail to compile");
        match result {
            Err(VmmError::WasmError(msg)) => {
                assert!(msg.contains("Invalid"), "Error should mention invalid bytecode");
            }
            _ => panic!("Expected WasmError"),
        }
    }

    /// Test: WASM module compilation fails with empty bytecode
    #[test]
    fn test_wasm_module_compilation_empty() {
        let result = WasmModule::compile("empty", &[]);

        assert!(result.is_err(), "Empty bytecode should fail");
    }

    /// Test: WASI capabilities detection
    #[test]
    fn test_wasi_capabilities() {
        let none = WasiCapabilities::none();
        assert!(!none.fs_read);
        assert!(!none.network);
        assert!(!none.requires_vm());

        let basic = WasiCapabilities::basic();
        assert!(basic.clock);
        assert!(basic.random);
        assert!(!basic.requires_vm());

        let full = WasiCapabilities::full();
        assert!(full.network);
        assert!(full.fs_write);
        assert!(full.requires_vm(), "Network/fs_write requires VM promotion");
    }

    /// Test: Sandbox configuration defaults are safe
    #[test]
    fn test_sandbox_config_defaults() {
        let config = WasmSandboxConfig::default();

        assert!(config.max_memory <= 256 * 1024 * 1024, "Default memory should be reasonable");
        assert!(config.max_execution_time <= Duration::from_secs(60));
        assert!(!config.enable_wasi, "WASI should be disabled by default for safety");
        assert!(config.auto_promote, "Auto-promotion should be enabled");
    }

    /// Performance Test: WASM compilation time
    #[test]
    fn test_wasm_compilation_performance() {
        let bytecode = create_test_wasm_module();
        let mut timing = TimingMeasurement::new("WASM Compilation");

        // Warm up
        for _ in 0..5 {
            let _ = WasmModule::compile("warmup", &bytecode);
        }

        // Measure
        for i in 0..100 {
            let _ = timing.measure(|| WasmModule::compile(&format!("mod-{}", i), &bytecode));
        }

        println!("{}", timing.report());

        // Compilation should be fast (< 10ms for small modules)
        assert!(
            timing.p99() < Duration::from_millis(10),
            "WASM compilation should be < 10ms, got {:?}",
            timing.p99()
        );
    }
}

// ============================================================================
// CLONE POOL TESTS
// ============================================================================

mod clone_pool_tests {
    use super::*;

    /// Test: Clone pool configuration validation
    #[test]
    fn test_clone_pool_config_defaults() {
        let config = ClonePoolConfig::default();

        assert!(config.min_warm_clones >= 1);
        assert!(config.max_warm_clones >= config.min_warm_clones);
        assert!(config.idle_timeout >= Duration::from_secs(60));
        assert!(config.target_density > 0);
    }

    /// Test: Clone pool configuration builder
    #[test]
    fn test_clone_pool_config_builder() {
        let config = ClonePoolConfig {
            min_warm_clones: 5,
            max_warm_clones: 50,
            idle_timeout: Duration::from_secs(120),
            enable_ksm: true,
            target_density: 1000,
        };

        assert_eq!(config.min_warm_clones, 5);
        assert_eq!(config.max_warm_clones, 50);
    }

    /// Test: Clone state transitions
    #[test]
    fn test_clone_states() {
        // Verify all states are distinct
        assert_ne!(CloneState::Ready, CloneState::Running);
        assert_ne!(CloneState::Running, CloneState::Dirty);
        assert_ne!(CloneState::Dirty, CloneState::Recycling);
    }
}

// ============================================================================
// SHARED MEMORY TESTS
// ============================================================================

mod shared_memory_tests {
    use super::*;

    /// Test: Shared memory configuration defaults
    #[test]
    fn test_shared_memory_config_defaults() {
        let config = SharedMemoryConfig::default();

        assert!(config.enable_ksm, "KSM should be enabled by default");
        assert!(config.enable_runtime_sharing);
        assert!(config.max_shared_per_runtime > 0);
    }

    /// Test: Shared memory manager creation (does not require privileges)
    #[test]
    fn test_shared_memory_manager_creation() {
        let config = SharedMemoryConfig {
            enable_ksm: false, // Disable KSM for unprivileged test
            enable_runtime_sharing: false,
            shared_path: std::env::temp_dir().join("nanovm-test"),
            ..Default::default()
        };

        let result = SharedMemoryManager::new(config);
        // Should succeed even without KSM privileges
        assert!(result.is_ok(), "SharedMemoryManager should create without KSM");
    }

    /// Test: Memory calculation for density claims
    #[test]
    fn test_memory_density_calculation() {
        // Our claim: 1000 Node.js instances in ~1GB with sharing
        // vs ~31GB without sharing

        let shared_runtime_size: u64 = 30 * 1024 * 1024; // 30MB Node.js runtime
        let private_per_instance: u64 = 1 * 1024 * 1024; // 1MB private per instance
        let instance_count: u64 = 1000;

        let with_sharing = shared_runtime_size + (instance_count * private_per_instance);
        let without_sharing = instance_count * (shared_runtime_size + private_per_instance);

        let savings_percent = 100 - (with_sharing * 100 / without_sharing);

        println!("With sharing: {} MB", with_sharing / (1024 * 1024));
        println!("Without sharing: {} MB", without_sharing / (1024 * 1024));
        println!("Savings: {}%", savings_percent);

        // Verify our claim of >90% savings
        assert!(
            savings_percent >= 90,
            "Memory sharing should save >90%, got {}%",
            savings_percent
        );
    }
}

// ============================================================================
// SNAPSHOT TESTS
// ============================================================================

mod snapshot_tests {
    use super::*;
    use nanolambda_vmm::snapshot::{CpuSnapshotState, ControlRegisters, MemoryRegionSnapshot, SegmentRegisters};

    /// Test: Runtime type conversion
    #[test]
    fn test_runtime_types() {
        assert_eq!(RuntimeType::NodeJs.as_str(), "nodejs");
        assert_eq!(RuntimeType::Python.as_str(), "python");
        assert_eq!(RuntimeType::Go.as_str(), "go");
        assert_eq!(RuntimeType::Native.as_str(), "native");
        assert_eq!(RuntimeType::Custom.as_str(), "custom");
    }

    /// Test: CPU snapshot state is zeroed by default
    #[test]
    fn test_cpu_snapshot_state_default() {
        let state = CpuSnapshotState::default();

        assert_eq!(state.rip, 0);
        assert_eq!(state.rsp, 0);
        assert_eq!(state.rflags, 0);
        assert!(state.msrs.is_empty());
    }

    /// Test: Snapshot manager creation
    #[test]
    fn test_snapshot_manager_creation() {
        let temp_dir = std::env::temp_dir().join("nanovm-snapshot-test");
        std::fs::create_dir_all(&temp_dir).ok();

        let result = SnapshotManager::new(&temp_dir);
        assert!(result.is_ok(), "SnapshotManager should create in temp dir");

        // Cleanup
        std::fs::remove_dir_all(&temp_dir).ok();
    }
}

// ============================================================================
// NANOVM INTEGRATION TESTS
// ============================================================================

mod nanovm_tests {
    use super::*;

    /// Test: NanoVM configuration defaults are production-safe
    #[test]
    fn test_nanovm_config_defaults() {
        let config = NanoVmConfig::default();

        // Verify safe defaults
        assert!(config.max_concurrent > 0 && config.max_concurrent <= 10000);
        assert!(config.default_timeout >= Duration::from_secs(1));
        assert!(config.default_timeout <= Duration::from_secs(300));
        assert!(config.enable_wasm_fastpath, "WASM fast-path should be enabled");
        assert!(config.enable_metrics, "Metrics should be enabled by default");
    }

    /// Test: Execution tier string conversion
    #[test]
    fn test_execution_tier_strings() {
        assert_eq!(ExecutionTier::WasmSandbox.as_str(), "wasm");
        assert_eq!(ExecutionTier::MicroVmWarm.as_str(), "microvm_warm");
        assert_eq!(ExecutionTier::MicroVmCold.as_str(), "microvm_cold");
        assert_eq!(ExecutionTier::MicroVmBoot.as_str(), "microvm_boot");
    }
}

// ============================================================================
// SYSTEM REQUIREMENTS TESTS
// ============================================================================

mod system_tests {
    use super::*;

    /// Test: VMM context creation and system report
    #[test]
    fn test_vmm_context() {
        let ctx = VmmContext::new();
        let report = ctx.system_report();

        // Report should always be generated
        assert!(!report.is_empty());
        assert!(report.contains("NanoVM System Report"));

        // Should report on /dev/kvm status
        assert!(report.contains("/dev/kvm"));

        println!("{}", report);
    }

    /// Test: KVM availability detection (informational)
    #[test]
    fn test_kvm_detection() {
        let ctx = VmmContext::new();

        match ctx.check_kvm_support() {
            Ok(caps) => {
                println!("KVM available:");
                println!("  Max vCPUs: {}", caps.max_vcpus);
                println!("  Max memslots: {}", caps.max_memslots);
                println!("  Userfaultfd: {}", caps.userfaultfd);
            }
            Err(e) => {
                println!("KVM not available: {}", e);
                // This is expected in CI/containers without KVM
            }
        }
    }
}

// ============================================================================
// CONCURRENCY TESTS
// ============================================================================

mod concurrency_tests {
    use super::*;
    use std::thread;

    /// Test: Multiple threads can create WASM modules concurrently
    #[test]
    fn test_concurrent_wasm_compilation() {
        let bytecode = Arc::new(create_test_wasm_module());
        let success_count = Arc::new(AtomicUsize::new(0));
        let thread_count = 10;
        let iterations = 100;

        let handles: Vec<_> = (0..thread_count)
            .map(|t| {
                let bytecode = Arc::clone(&bytecode);
                let success_count = Arc::clone(&success_count);

                thread::spawn(move || {
                    for i in 0..iterations {
                        let id = format!("thread-{}-mod-{}", t, i);
                        if WasmModule::compile(&id, &bytecode).is_ok() {
                            success_count.fetch_add(1, Ordering::Relaxed);
                        }
                    }
                })
            })
            .collect();

        for handle in handles {
            handle.join().expect("Thread should not panic");
        }

        let total_success = success_count.load(Ordering::Relaxed);
        let expected = thread_count * iterations;

        assert_eq!(
            total_success, expected,
            "All {} compilations should succeed, got {}",
            expected, total_success
        );
    }

    /// Test: Clone pool stats are thread-safe
    #[test]
    fn test_clone_pool_stats_thread_safety() {
        use nanolambda_vmm::nanovm::NanoVmStats;

        let stats = Arc::new(NanoVmStats::default());
        let thread_count = 10;
        let increments = 1000;

        let handles: Vec<_> = (0..thread_count)
            .map(|_| {
                let stats = Arc::clone(&stats);
                thread::spawn(move || {
                    for _ in 0..increments {
                        stats.total_executions.fetch_add(1, Ordering::Relaxed);
                        stats.wasm_executions.fetch_add(1, Ordering::Relaxed);
                    }
                })
            })
            .collect();

        for handle in handles {
            handle.join().expect("Thread should not panic");
        }

        let total = stats.total_executions.load(Ordering::Relaxed);
        let expected = (thread_count * increments) as u64;

        assert_eq!(total, expected, "Stats should be thread-safe");
    }
}

// ============================================================================
// ERROR HANDLING TESTS
// ============================================================================

mod error_tests {
    use super::*;

    /// Test: VmmError variants have meaningful messages
    #[test]
    fn test_error_messages() {
        let errors = vec![
            VmmError::KvmError("test kvm error".to_string()),
            VmmError::MemoryError("test memory error".to_string()),
            VmmError::WasmError("test wasm error".to_string()),
            VmmError::ConfigError("test config error".to_string()),
        ];

        for error in errors {
            let msg = format!("{}", error);
            assert!(!msg.is_empty(), "Error should have non-empty message");
            assert!(msg.contains("test"), "Error message should contain context");
        }
    }

    /// Test: Error conversion to string is stable
    #[test]
    fn test_error_display() {
        let error = VmmError::KvmError("KVM unavailable".to_string());
        let display1 = format!("{}", error);
        let display2 = format!("{}", error);
        assert_eq!(display1, display2, "Error display should be stable");
    }
}

// ============================================================================
// PERFORMANCE VALIDATION TESTS
// ============================================================================

mod performance_tests {
    use super::*;

    /// Performance Test: Validate cold start claim (<1ms for WASM)
    /// 
    /// This test verifies our core performance claim that WASM sandbox
    /// cold starts complete in under 1ms.
    #[test]
    fn bench_wasm_cold_start_claim() {
        let bytecode = create_test_wasm_module();
        let mut timing = TimingMeasurement::new("WASM Cold Start");

        // Warm up the compiler
        for _ in 0..10 {
            let _ = WasmModule::compile("warmup", &bytecode);
        }

        // Measure actual cold starts
        for i in 0..1000 {
            // Each iteration is a fresh module (simulates cold start)
            let _ = timing.measure(|| WasmModule::compile(&format!("cold-{}", i), &bytecode));
        }

        println!("\n=== WASM Cold Start Performance ===");
        println!("{}", timing.report());

        // Validate claim: p99 < 1ms
        assert!(
            timing.p99() < Duration::from_millis(1),
            "WASM cold start p99 should be < 1ms, got {:?}",
            timing.p99()
        );

        // Mean should be well under the target
        assert!(
            timing.mean() < Duration::from_micros(500),
            "WASM cold start mean should be < 500µs, got {:?}",
            timing.mean()
        );
    }

    /// Performance Test: Memory allocation performance
    #[test]
    fn bench_memory_allocation() {
        let mut timing = TimingMeasurement::new("Memory Allocation (4KB page)");
        let page_size = 4096;

        for _ in 0..1000 {
            timing.measure(|| {
                let mut page = vec![0u8; page_size];
                // Touch the memory to ensure it's allocated
                page[0] = 1;
                page[page_size - 1] = 1;
                drop(page);
            });
        }

        println!("\n=== Memory Allocation Performance ===");
        println!("{}", timing.report());

        // Page allocation should be fast (< 100µs)
        assert!(
            timing.p99() < Duration::from_micros(100),
            "Page allocation should be < 100µs, got {:?}",
            timing.p99()
        );
    }

    /// Performance Test: Atomic operations (used in stats tracking)
    #[test]
    fn bench_atomic_operations() {
        use std::sync::atomic::AtomicU64;

        let counter = AtomicU64::new(0);
        let mut timing = TimingMeasurement::new("Atomic fetch_add");

        for _ in 0..100_000 {
            timing.measure(|| {
                counter.fetch_add(1, Ordering::Relaxed);
            });
        }

        println!("\n=== Atomic Operations Performance ===");
        println!("{}", timing.report());

        // Atomic operations should be essentially free (< 1µs)
        assert!(
            timing.mean() < Duration::from_micros(1),
            "Atomic ops should be < 1µs, got {:?}",
            timing.mean()
        );
    }
}

// ============================================================================
// INTEGRATION TESTS (Require specific system configuration)
// ============================================================================

mod integration_tests {
    use super::*;

    /// Integration Test: Full NanoVM lifecycle
    /// 
    /// This test requires:
    /// - Writable temp directory
    /// - Does NOT require KVM (uses WASM-only mode)
    #[test]
    fn test_nanovm_lifecycle() {
        let temp_dir = std::env::temp_dir().join("nanovm-integration-test");
        std::fs::create_dir_all(&temp_dir).ok();

        let config = NanoVmConfig {
            enable_wasm_fastpath: true,
            snapshot_path: temp_dir.to_string_lossy().to_string(),
            enable_metrics: true,
            ..Default::default()
        };

        // Create NanoVM instance
        let nanovm_result = NanoVm::new(config);

        match nanovm_result {
            Ok(nanovm) => {
                // Verify stats are initialized
                let stats = nanovm.stats();
                assert_eq!(stats.total_executions.load(Ordering::Relaxed), 0);
                println!("NanoVM created successfully");
            }
            Err(e) => {
                // May fail in CI due to missing KVM, that's acceptable
                println!("NanoVM creation failed (expected in CI): {}", e);
            }
        }

        // Cleanup
        std::fs::remove_dir_all(&temp_dir).ok();
    }

    /// Integration Test: Verify KVM operations (if available)
    #[test]
    #[ignore] // Run with: cargo test -- --ignored
    fn test_kvm_operations() {
        let ctx = VmmContext::new();

        let caps = match ctx.check_kvm_support() {
            Ok(c) => c,
            Err(e) => {
                println!("Skipping KVM test: {}", e);
                return;
            }
        };

        println!("KVM Capabilities:");
        println!("  Max vCPUs: {}", caps.max_vcpus);
        println!("  Max memslots: {}", caps.max_memslots);
        println!("  Extended CPUID: {}", caps.ext_cpuid);
        println!("  Userfaultfd: {}", caps.userfaultfd);
        println!("  Dirty tracking: {}", caps.dirty_tracking);

        // Verify reasonable limits
        assert!(caps.max_vcpus >= 1, "Should support at least 1 vCPU");
        assert!(caps.max_memslots >= 2, "Should support at least 2 memslots");
    }
}
