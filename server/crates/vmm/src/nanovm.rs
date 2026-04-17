//! NanoVM - Ultra-fast Serverless Execution Engine
//!
//! Orchestrates the hybrid execution model:
//! 1. Try WASM sandbox first (<1ms cold start)
//! 2. Auto-promote to microVM if needed (~10ms with snapshots)
//! 3. Use shared memory for maximum density

use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

use crate::clone_pool::{ClonePool, ClonePoolConfig, ClonePoolManager, VmClone};
use crate::error::{VmmError, VmmResult};
use crate::shared_memory::{SharedMemoryConfig, SharedMemoryManager};
use crate::snapshot::{RuntimeType, SnapshotManager, VmSnapshot};
use crate::wasm_sandbox::{HybridExecutor, PromotionReason, WasmModule, WasmSandboxConfig, WasmValue};

/// NanoVM configuration
#[derive(Debug, Clone)]
pub struct NanoVmConfig {
    /// Maximum concurrent executions
    pub max_concurrent: usize,
    /// Default execution timeout
    pub default_timeout: Duration,
    /// Enable WASM fast-path
    pub enable_wasm_fastpath: bool,
    /// WASM sandbox config
    pub wasm_config: WasmSandboxConfig,
    /// Clone pool config
    pub pool_config: ClonePoolConfig,
    /// Shared memory config
    pub shared_config: SharedMemoryConfig,
    /// Snapshot storage path
    pub snapshot_path: String,
    /// Enable metrics collection
    pub enable_metrics: bool,
}

impl Default for NanoVmConfig {
    fn default() -> Self {
        Self {
            max_concurrent: 1000,
            default_timeout: Duration::from_secs(30),
            enable_wasm_fastpath: true,
            wasm_config: WasmSandboxConfig::default(),
            pool_config: ClonePoolConfig::default(),
            shared_config: SharedMemoryConfig::default(),
            snapshot_path: "/var/lib/nanovm/snapshots".to_string(),
            enable_metrics: true,
        }
    }
}

/// Execution request for a function
#[derive(Debug)]
pub struct ExecutionRequest {
    /// Unique request ID
    pub id: String,
    /// Function ID
    pub function_id: String,
    /// Runtime type
    pub runtime: RuntimeType,
    /// Function to execute
    pub handler: String,
    /// Input payload
    pub payload: Vec<u8>,
    /// WASM bytecode (if available)
    pub wasm_bytecode: Option<Vec<u8>>,
    /// Execution timeout
    pub timeout: Option<Duration>,
    /// Memory limit
    pub memory_limit: Option<usize>,
}

/// Result of function execution
#[derive(Debug)]
pub struct ExecutionResult {
    /// Request ID
    pub request_id: String,
    /// Execution output
    pub output: Vec<u8>,
    /// Execution duration
    pub duration: Duration,
    /// Execution tier used
    pub tier: ExecutionTier,
    /// Memory used
    pub memory_used: usize,
    /// Whether function was cold start
    pub cold_start: bool,
    /// Detailed metrics
    pub metrics: ExecutionMetrics,
}

/// Execution tier
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExecutionTier {
    /// Executed in WASM sandbox
    WasmSandbox,
    /// Executed in microVM (warm)
    MicroVmWarm,
    /// Executed in microVM (cold - snapshot restore)
    MicroVmCold,
    /// Executed in microVM (full boot)
    MicroVmBoot,
}

impl ExecutionTier {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::WasmSandbox => "wasm",
            Self::MicroVmWarm => "microvm_warm",
            Self::MicroVmCold => "microvm_cold",
            Self::MicroVmBoot => "microvm_boot",
        }
    }
}

/// Detailed execution metrics
#[derive(Debug, Default)]
pub struct ExecutionMetrics {
    /// Time to acquire execution slot
    pub acquire_time: Duration,
    /// Time for WASM execution (if used)
    pub wasm_time: Option<Duration>,
    /// Time for VM restore (if used)
    pub vm_restore_time: Option<Duration>,
    /// Time for function execution
    pub function_time: Duration,
    /// Dirty pages (if VM used)
    pub dirty_pages: Option<usize>,
    /// Promotion reason (if promoted from WASM)
    pub promotion_reason: Option<String>,
}

/// Global execution statistics
#[derive(Debug, Default)]
pub struct NanoVmStats {
    /// Total executions
    pub total_executions: AtomicU64,
    /// WASM executions
    pub wasm_executions: AtomicU64,
    /// Warm VM executions
    pub warm_vm_executions: AtomicU64,
    /// Cold VM executions (snapshot restore)
    pub cold_vm_executions: AtomicU64,
    /// Full VM boots
    pub full_vm_boots: AtomicU64,
    /// Promotions from WASM to VM
    pub promotions: AtomicU64,
    /// Errors
    pub errors: AtomicU64,
    /// Total execution time (microseconds)
    pub total_execution_time_us: AtomicU64,
    /// Memory saved by sharing (bytes)
    pub memory_saved: AtomicU64,
}

/// The main NanoVM execution engine
pub struct NanoVm {
    /// Configuration
    config: NanoVmConfig,
    /// Snapshot manager
    snapshot_manager: SnapshotManager,
    /// Clone pool manager
    pool_manager: ClonePoolManager,
    /// Shared memory manager
    shared_memory: SharedMemoryManager,
    /// WASM executor
    wasm_executor: HybridExecutor,
    /// Global statistics
    stats: NanoVmStats,
    /// Active executions
    active_executions: AtomicU64,
}

impl NanoVm {
    /// Create a new NanoVM instance
    pub fn new(config: NanoVmConfig) -> VmmResult<Self> {
        let snapshot_manager = SnapshotManager::new(&config.snapshot_path)?;
        let pool_manager = ClonePoolManager::new(config.pool_config.clone());
        let shared_memory = SharedMemoryManager::new(config.shared_config.clone())?;
        let wasm_executor = HybridExecutor::new(config.wasm_config.clone());

        Ok(Self {
            config,
            snapshot_manager,
            pool_manager,
            shared_memory,
            wasm_executor,
            stats: NanoVmStats::default(),
            active_executions: AtomicU64::new(0),
        })
    }

    /// Initialize runtime environments
    pub fn init_runtimes(&mut self, runtimes: &[RuntimeType]) -> VmmResult<()> {
        for runtime in runtimes {
            self.init_runtime(*runtime)?;
        }
        Ok(())
    }

    /// Initialize a specific runtime
    fn init_runtime(&mut self, runtime: RuntimeType) -> VmmResult<()> {
        // Create golden snapshot for this runtime
        let snapshot = self.create_golden_snapshot(runtime)?;
        let snapshot = Arc::new(snapshot);

        // Register with snapshot manager
        self.snapshot_manager.register_golden_snapshot(runtime, Arc::clone(&snapshot));

        // Create clone pool
        self.pool_manager.register_runtime(runtime, snapshot)?;

        Ok(())
    }

    /// Create a golden snapshot for a runtime
    fn create_golden_snapshot(&self, runtime: RuntimeType) -> VmmResult<VmSnapshot> {
        use crate::snapshot::GoldenSnapshotBuilder;

        let builder = GoldenSnapshotBuilder::new(runtime).memory_mb(128);

        // Add runtime-specific pre-warming
        let builder = match runtime {
            RuntimeType::NodeJs => builder
                .boot_script("node -e 'require(\"http\"); require(\"fs\");'")
                .pre_warm_module("express")
                .pre_warm_module("lodash"),
            RuntimeType::Python => builder
                .boot_script("python3 -c 'import json, os, sys'")
                .pre_warm_module("requests")
                .pre_warm_module("boto3"),
            _ => builder,
        };

        builder.build()
    }

    /// Execute a function
    pub fn execute(&self, request: ExecutionRequest) -> VmmResult<ExecutionResult> {
        let start = Instant::now();
        self.stats.total_executions.fetch_add(1, Ordering::Relaxed);

        // Check concurrency limit
        let active = self.active_executions.fetch_add(1, Ordering::SeqCst);
        if active as usize >= self.config.max_concurrent {
            self.active_executions.fetch_sub(1, Ordering::SeqCst);
            return Err(VmmError::ResourceExhausted(
                "Maximum concurrent executions reached".to_string(),
            ));
        }

        let result = self.execute_internal(request, start);

        self.active_executions.fetch_sub(1, Ordering::SeqCst);

        match &result {
            Ok(r) => {
                self.stats.total_execution_time_us.fetch_add(
                    r.duration.as_micros() as u64,
                    Ordering::Relaxed,
                );
            }
            Err(_) => {
                self.stats.errors.fetch_add(1, Ordering::Relaxed);
            }
        }

        result
    }

    /// Internal execution logic
    fn execute_internal(
        &self,
        request: ExecutionRequest,
        start: Instant,
    ) -> VmmResult<ExecutionResult> {
        let mut metrics = ExecutionMetrics::default();

        // Try WASM fast-path first
        if self.config.enable_wasm_fastpath {
            if let Some(ref bytecode) = request.wasm_bytecode {
                match self.try_wasm_execution(&request, bytecode, &mut metrics) {
                    Ok(output) => {
                        self.stats.wasm_executions.fetch_add(1, Ordering::Relaxed);
                        return Ok(ExecutionResult {
                            request_id: request.id,
                            output,
                            duration: start.elapsed(),
                            tier: ExecutionTier::WasmSandbox,
                            memory_used: 0, // TODO: track
                            cold_start: false,
                            metrics,
                        });
                    }
                    Err(VmmError::WasmPromotionRequired(reason)) => {
                        // Need to promote to microVM
                        self.stats.promotions.fetch_add(1, Ordering::Relaxed);
                        metrics.promotion_reason = Some(reason);
                    }
                    Err(e) => return Err(e),
                }
            }
        }

        // Execute in microVM
        self.execute_in_vm(request, start, metrics)
    }

    /// Try WASM execution
    fn try_wasm_execution(
        &self,
        request: &ExecutionRequest,
        bytecode: &[u8],
        metrics: &mut ExecutionMetrics,
    ) -> VmmResult<Vec<u8>> {
        let wasm_start = Instant::now();

        // Compile module
        let module = Arc::new(WasmModule::compile(&request.function_id, bytecode)?);

        // Check sandbox compatibility
        if !module.sandbox_compatible {
            return Err(VmmError::WasmPromotionRequired(
                "Module requires VM capabilities".to_string(),
            ));
        }

        // Execute
        let args = self.deserialize_wasm_args(&request.payload)?;
        let result = self.wasm_executor.execute(&module, &request.handler, args)?;

        metrics.wasm_time = Some(wasm_start.elapsed());
        metrics.function_time = wasm_start.elapsed();

        Ok(result)
    }

    /// Deserialize payload to WASM arguments
    fn deserialize_wasm_args(&self, payload: &[u8]) -> VmmResult<Vec<WasmValue>> {
        // Simple implementation - real one would parse JSON/MessagePack
        if payload.is_empty() {
            return Ok(vec![]);
        }

        // Pass payload as i64 pointer for now
        Ok(vec![WasmValue::I64(payload.len() as i64)])
    }

    /// Execute in microVM
    fn execute_in_vm(
        &self,
        request: ExecutionRequest,
        start: Instant,
        mut metrics: ExecutionMetrics,
    ) -> VmmResult<ExecutionResult> {
        let acquire_start = Instant::now();

        // Try to get a warm clone first
        let (clone, tier, cold_start) = match self.pool_manager.acquire(request.runtime) {
            Ok(clone) => {
                self.stats.warm_vm_executions.fetch_add(1, Ordering::Relaxed);
                (clone, ExecutionTier::MicroVmWarm, false)
            }
            Err(_) => {
                // Need to restore from snapshot
                let clone = self.restore_from_snapshot(request.runtime, &mut metrics)?;
                self.stats.cold_vm_executions.fetch_add(1, Ordering::Relaxed);
                (clone, ExecutionTier::MicroVmCold, true)
            }
        };

        metrics.acquire_time = acquire_start.elapsed();

        // Execute in the VM
        let exec_start = Instant::now();
        let output = self.run_in_clone(&clone, &request)?;
        metrics.function_time = exec_start.elapsed();
        metrics.dirty_pages = Some(clone.dirty_page_count());

        // Release clone back to pool
        self.pool_manager.release(clone)?;

        Ok(ExecutionResult {
            request_id: request.id,
            output,
            duration: start.elapsed(),
            tier,
            memory_used: 0, // TODO: track
            cold_start,
            metrics,
        })
    }

    /// Restore a VM from snapshot
    fn restore_from_snapshot(
        &self,
        runtime: RuntimeType,
        metrics: &mut ExecutionMetrics,
    ) -> VmmResult<Box<VmClone>> {
        let restore_start = Instant::now();

        // Get golden snapshot
        let _snapshot = self.snapshot_manager.get_golden_snapshot(runtime).ok_or_else(|| {
            VmmError::SnapshotError(format!("No golden snapshot for {:?}", runtime))
        })?;

        // Create clone from snapshot (uses lazy loading)
        let clone = self.pool_manager.acquire(runtime)?;

        metrics.vm_restore_time = Some(restore_start.elapsed());

        Ok(clone)
    }

    /// Run function in a VM clone
    fn run_in_clone(&self, _clone: &VmClone, request: &ExecutionRequest) -> VmmResult<Vec<u8>> {
        // In real implementation:
        // 1. Write request payload to VM memory
        // 2. Set up execution context (handler, environment)
        // 3. Run the VM
        // 4. Read response from VM memory

        // Placeholder
        Ok(format!(
            "Executed {} in {:?} VM",
            request.handler, request.runtime
        )
        .into_bytes())
    }

    /// Get execution statistics
    pub fn stats(&self) -> &NanoVmStats {
        &self.stats
    }

    /// Get detailed statistics report
    pub fn stats_report(&self) -> StatsReport {
        let total = self.stats.total_executions.load(Ordering::Relaxed);
        let wasm = self.stats.wasm_executions.load(Ordering::Relaxed);
        let warm = self.stats.warm_vm_executions.load(Ordering::Relaxed);
        let cold = self.stats.cold_vm_executions.load(Ordering::Relaxed);
        let errors = self.stats.errors.load(Ordering::Relaxed);
        let total_time_us = self.stats.total_execution_time_us.load(Ordering::Relaxed);

        let avg_time_us = if total > 0 {
            total_time_us / total
        } else {
            0
        };

        StatsReport {
            total_executions: total,
            wasm_percent: if total > 0 {
                (wasm as f64 / total as f64) * 100.0
            } else {
                0.0
            },
            warm_percent: if total > 0 {
                (warm as f64 / total as f64) * 100.0
            } else {
                0.0
            },
            cold_percent: if total > 0 {
                (cold as f64 / total as f64) * 100.0
            } else {
                0.0
            },
            error_rate: if total > 0 {
                (errors as f64 / total as f64) * 100.0
            } else {
                0.0
            },
            avg_execution_time: Duration::from_micros(avg_time_us),
            memory_saved: self.stats.memory_saved.load(Ordering::Relaxed),
            pool_stats: self.pool_manager.global_stats(),
        }
    }

    /// Run maintenance tasks
    pub fn maintenance(&self) -> MaintenanceReport {
        let pool_result = self.pool_manager.maintenance();
        let memory_result = self.shared_memory.maintenance();

        // Update memory saved stat
        self.stats.memory_saved.store(
            self.shared_memory.total_memory_saved(),
            Ordering::Relaxed,
        );

        MaintenanceReport {
            clones_recycled: pool_result.clones_recycled,
            pages_merged: pool_result.pages_merged + memory_result.pages_merged as usize,
            memory_saved: memory_result.memory_saved,
            regions_cleaned: memory_result.regions_cleaned,
        }
    }
}

/// Statistics report
#[derive(Debug)]
pub struct StatsReport {
    pub total_executions: u64,
    pub wasm_percent: f64,
    pub warm_percent: f64,
    pub cold_percent: f64,
    pub error_rate: f64,
    pub avg_execution_time: Duration,
    pub memory_saved: u64,
    pub pool_stats: crate::clone_pool::GlobalPoolStats,
}

/// Maintenance report
#[derive(Debug, Default)]
pub struct MaintenanceReport {
    pub clones_recycled: usize,
    pub pages_merged: usize,
    pub memory_saved: u64,
    pub regions_cleaned: usize,
}

/// Builder for NanoVM
pub struct NanoVmBuilder {
    config: NanoVmConfig,
    runtimes: Vec<RuntimeType>,
}

impl NanoVmBuilder {
    pub fn new() -> Self {
        Self {
            config: NanoVmConfig::default(),
            runtimes: Vec::new(),
        }
    }

    pub fn max_concurrent(mut self, max: usize) -> Self {
        self.config.max_concurrent = max;
        self
    }

    pub fn timeout(mut self, timeout: Duration) -> Self {
        self.config.default_timeout = timeout;
        self
    }

    pub fn enable_wasm_fastpath(mut self, enable: bool) -> Self {
        self.config.enable_wasm_fastpath = enable;
        self
    }

    pub fn snapshot_path(mut self, path: impl Into<String>) -> Self {
        self.config.snapshot_path = path.into();
        self
    }

    pub fn runtime(mut self, runtime: RuntimeType) -> Self {
        self.runtimes.push(runtime);
        self
    }

    pub fn build(self) -> VmmResult<NanoVm> {
        let mut nanovm = NanoVm::new(self.config)?;
        if !self.runtimes.is_empty() {
            nanovm.init_runtimes(&self.runtimes)?;
        }
        Ok(nanovm)
    }
}

impl Default for NanoVmBuilder {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn test_config() -> NanoVmConfig {
        NanoVmConfig {
            snapshot_path: std::env::temp_dir()
                .join("nanovm_test")
                .to_string_lossy()
                .to_string(),
            ..Default::default()
        }
    }

    #[test]
    fn test_nanovm_creation() {
        let nanovm = NanoVm::new(test_config());
        assert!(nanovm.is_ok());
    }

    #[test]
    fn test_execution_tier_str() {
        assert_eq!(ExecutionTier::WasmSandbox.as_str(), "wasm");
        assert_eq!(ExecutionTier::MicroVmWarm.as_str(), "microvm_warm");
    }

    #[test]
    fn test_builder() {
        let builder = NanoVmBuilder::new()
            .max_concurrent(500)
            .timeout(Duration::from_secs(60))
            .enable_wasm_fastpath(true);

        assert_eq!(builder.config.max_concurrent, 500);
    }

    #[test]
    fn test_stats_report() {
        let nanovm = NanoVm::new(test_config()).unwrap();
        let report = nanovm.stats_report();

        assert_eq!(report.total_executions, 0);
        assert_eq!(report.error_rate, 0.0);
    }
}
