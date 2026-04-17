//! WASM Fast-Path Sandbox
//!
//! Provides a lightweight WebAssembly sandbox for simple functions that don't need
//! full VM isolation. Functions start in WASM and are automatically promoted to
//! microVM if they require syscalls or other capabilities.

use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

use crate::error::{VmmError, VmmResult};

/// Configuration for the WASM sandbox
#[derive(Debug, Clone)]
pub struct WasmSandboxConfig {
    /// Maximum memory in bytes
    pub max_memory: usize,
    /// Maximum execution time
    pub max_execution_time: Duration,
    /// Maximum stack size
    pub max_stack_size: usize,
    /// Enable WASI (filesystem, env, etc.)
    pub enable_wasi: bool,
    /// Allowed WASI capabilities
    pub wasi_caps: WasiCapabilities,
    /// Auto-promote to microVM on capability request
    pub auto_promote: bool,
}

impl Default for WasmSandboxConfig {
    fn default() -> Self {
        Self {
            max_memory: 128 * 1024 * 1024, // 128 MB
            max_execution_time: Duration::from_secs(30),
            max_stack_size: 1024 * 1024, // 1 MB
            enable_wasi: false,
            wasi_caps: WasiCapabilities::none(),
            auto_promote: true,
        }
    }
}

/// WASI capability flags
#[derive(Debug, Clone, Default)]
pub struct WasiCapabilities {
    /// Read from preopened directories
    pub fs_read: bool,
    /// Write to preopened directories
    pub fs_write: bool,
    /// Access environment variables
    pub env_vars: bool,
    /// Network sockets
    pub network: bool,
    /// Random number generation
    pub random: bool,
    /// Clock/time access
    pub clock: bool,
}

impl WasiCapabilities {
    pub fn none() -> Self {
        Self::default()
    }

    pub fn basic() -> Self {
        Self {
            env_vars: true,
            random: true,
            clock: true,
            ..Default::default()
        }
    }

    pub fn full() -> Self {
        Self {
            fs_read: true,
            fs_write: true,
            env_vars: true,
            network: true,
            random: true,
            clock: true,
        }
    }

    /// Check if any capability requires promotion to microVM
    pub fn requires_vm(&self) -> bool {
        self.fs_write || self.network
    }
}

/// Reason for promoting from WASM to microVM
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PromotionReason {
    /// Requested filesystem write
    FilesystemWrite,
    /// Requested network access
    NetworkAccess,
    /// Requested raw syscall
    RawSyscall(i32),
    /// Exceeded memory limit
    MemoryExceeded,
    /// Requested native code execution
    NativeCode,
    /// Explicit request by function
    ExplicitRequest,
}

/// Result of WASM execution
#[derive(Debug)]
pub enum WasmExecutionResult {
    /// Execution completed successfully
    Success(WasmOutput),
    /// Needs promotion to microVM
    NeedsPromotion(PromotionReason),
    /// Execution failed
    Error(WasmError),
}

/// Output from successful WASM execution
#[derive(Debug)]
pub struct WasmOutput {
    /// Return value (serialized)
    pub result: Vec<u8>,
    /// Execution duration
    pub duration: Duration,
    /// Peak memory usage
    pub peak_memory: usize,
    /// CPU cycles used (if available)
    pub cpu_cycles: Option<u64>,
}

/// WASM execution error
#[derive(Debug)]
pub enum WasmError {
    /// Compilation failed
    CompilationError(String),
    /// Runtime trap
    Trap(String),
    /// Timeout exceeded
    Timeout,
    /// Out of memory
    OutOfMemory,
    /// Invalid input
    InvalidInput(String),
}

/// A compiled WASM module ready for execution
pub struct WasmModule {
    /// Module identifier
    pub id: String,
    /// Original WASM bytecode size
    pub bytecode_size: usize,
    /// Detected capabilities needed by this module
    pub required_caps: WasiCapabilities,
    /// Whether this module can run in WASM sandbox
    pub sandbox_compatible: bool,
    /// Compiled module (would be wasmtime::Module in real impl)
    compiled: CompiledModule,
}

/// Placeholder for compiled WASM module
struct CompiledModule {
    /// Function exports
    exports: Vec<String>,
    /// Memory requirements
    min_memory_pages: u32,
    max_memory_pages: Option<u32>,
}

impl WasmModule {
    /// Compile a WASM module from bytecode
    pub fn compile(id: impl Into<String>, bytecode: &[u8]) -> VmmResult<Self> {
        let id = id.into();

        // Validate WASM magic bytes
        if bytecode.len() < 8 || &bytecode[0..4] != b"\0asm" {
            return Err(VmmError::WasmError("Invalid WASM bytecode".to_string()));
        }

        // Analyze module for capabilities
        let (required_caps, sandbox_compatible) = Self::analyze_capabilities(bytecode)?;

        // Compile module (simplified)
        let compiled = CompiledModule {
            exports: vec!["_start".to_string(), "main".to_string()],
            min_memory_pages: 1,
            max_memory_pages: Some(256), // 16 MB max
        };

        Ok(Self {
            id,
            bytecode_size: bytecode.len(),
            required_caps,
            sandbox_compatible,
            compiled,
        })
    }

    /// Analyze WASM module for required capabilities
    fn analyze_capabilities(bytecode: &[u8]) -> VmmResult<(WasiCapabilities, bool)> {
        // In real implementation, this would parse WASM imports to detect:
        // - WASI imports (fd_read, fd_write, etc.)
        // - Network imports
        // - Other system calls

        // For now, assume basic capabilities
        let caps = WasiCapabilities::basic();
        let sandbox_compatible = !caps.requires_vm();

        Ok((caps, sandbox_compatible))
    }

    /// Get exported function names
    pub fn exports(&self) -> &[String] {
        &self.compiled.exports
    }
}

/// WASM sandbox instance
pub struct WasmSandbox {
    /// Configuration
    config: WasmSandboxConfig,
    /// The module being executed
    module: Arc<WasmModule>,
    /// Current memory usage
    memory_usage: AtomicU64,
    /// Whether execution has started
    started: AtomicBool,
    /// Promotion request (if any)
    promotion_request: std::sync::Mutex<Option<PromotionReason>>,
    /// Host function bindings
    host_functions: HashMap<String, HostFunction>,
}

/// A host function callable from WASM
type HostFunction = Box<dyn Fn(&[WasmValue]) -> Result<Vec<WasmValue>, String> + Send + Sync>;

/// WASM value types
#[derive(Debug, Clone)]
pub enum WasmValue {
    I32(i32),
    I64(i64),
    F32(f32),
    F64(f64),
}

impl WasmSandbox {
    /// Create a new sandbox for a module
    pub fn new(module: Arc<WasmModule>, config: WasmSandboxConfig) -> VmmResult<Self> {
        if !module.sandbox_compatible && !config.auto_promote {
            return Err(VmmError::WasmError(
                "Module requires capabilities not available in sandbox".to_string(),
            ));
        }

        Ok(Self {
            config,
            module,
            memory_usage: AtomicU64::new(0),
            started: AtomicBool::new(false),
            promotion_request: std::sync::Mutex::new(None),
            host_functions: Self::create_default_host_functions(),
        })
    }

    /// Create default host function bindings
    fn create_default_host_functions() -> HashMap<String, HostFunction> {
        let mut funcs: HashMap<String, HostFunction> = HashMap::new();

        // Console logging
        funcs.insert(
            "console_log".to_string(),
            Box::new(|args| {
                for arg in args {
                    match arg {
                        WasmValue::I32(v) => print!("{}", v),
                        WasmValue::I64(v) => print!("{}", v),
                        WasmValue::F32(v) => print!("{}", v),
                        WasmValue::F64(v) => print!("{}", v),
                    }
                }
                println!();
                Ok(vec![])
            }),
        );

        // Get current time in milliseconds
        funcs.insert(
            "now_ms".to_string(),
            Box::new(|_| {
                let now = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .as_millis() as i64;
                Ok(vec![WasmValue::I64(now)])
            }),
        );

        // Random number
        funcs.insert(
            "random".to_string(),
            Box::new(|_| {
                // Simple pseudo-random for sandbox
                let r = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .subsec_nanos() as f64
                    / 1_000_000_000.0;
                Ok(vec![WasmValue::F64(r)])
            }),
        );

        funcs
    }

    /// Execute a function in the sandbox
    pub fn execute(
        &self,
        function: &str,
        args: Vec<WasmValue>,
    ) -> WasmExecutionResult {
        self.started.store(true, Ordering::SeqCst);
        let start = Instant::now();

        // Check for promotion request before execution
        if let Some(reason) = self.promotion_request.lock().unwrap().take() {
            return WasmExecutionResult::NeedsPromotion(reason);
        }

        // Check function exists
        if !self.module.exports().contains(&function.to_string()) {
            return WasmExecutionResult::Error(WasmError::InvalidInput(format!(
                "Function '{}' not found",
                function
            )));
        }

        // Simulated execution (real impl would use wasmtime)
        let result = self.simulate_execution(function, args);

        let duration = start.elapsed();

        // Check timeout
        if duration > self.config.max_execution_time {
            return WasmExecutionResult::Error(WasmError::Timeout);
        }

        match result {
            Ok(output) => WasmExecutionResult::Success(WasmOutput {
                result: output,
                duration,
                peak_memory: self.memory_usage.load(Ordering::Relaxed) as usize,
                cpu_cycles: None,
            }),
            Err(e) => WasmExecutionResult::Error(WasmError::Trap(e)),
        }
    }

    /// Simulate WASM execution (placeholder for real wasmtime integration)
    fn simulate_execution(&self, _function: &str, args: Vec<WasmValue>) -> Result<Vec<u8>, String> {
        // In real implementation, this would:
        // 1. Create wasmtime Store with resource limits
        // 2. Instantiate the module
        // 3. Call the exported function
        // 4. Marshal return values

        // For now, return serialized args as output
        let output = format!("executed with {} args", args.len());
        Ok(output.into_bytes())
    }

    /// Request promotion to microVM
    pub fn request_promotion(&self, reason: PromotionReason) {
        *self.promotion_request.lock().unwrap() = Some(reason);
    }

    /// Check if sandbox execution is still valid
    pub fn is_valid(&self) -> bool {
        self.promotion_request.lock().unwrap().is_none()
    }

    /// Get current memory usage
    pub fn memory_usage(&self) -> u64 {
        self.memory_usage.load(Ordering::Relaxed)
    }
}

/// WASM execution statistics
#[derive(Debug, Default)]
pub struct WasmStats {
    /// Total executions
    pub total_executions: AtomicU64,
    /// Successful executions (stayed in WASM)
    pub wasm_successes: AtomicU64,
    /// Promotions to microVM
    pub promotions: AtomicU64,
    /// Errors
    pub errors: AtomicU64,
    /// Total execution time in microseconds
    pub total_time_us: AtomicU64,
}

/// Hybrid executor that tries WASM first, promotes to VM if needed
pub struct HybridExecutor {
    /// WASM module cache
    module_cache: HashMap<String, Arc<WasmModule>>,
    /// Default sandbox config
    sandbox_config: WasmSandboxConfig,
    /// Statistics
    stats: WasmStats,
    /// Callback for VM promotion
    promote_callback: Option<Box<dyn Fn(PromotionReason) -> VmmResult<Vec<u8>> + Send + Sync>>,
}

impl HybridExecutor {
    /// Create a new hybrid executor
    pub fn new(sandbox_config: WasmSandboxConfig) -> Self {
        Self {
            module_cache: HashMap::new(),
            sandbox_config,
            stats: WasmStats::default(),
            promote_callback: None,
        }
    }

    /// Set the callback for VM promotion
    pub fn set_promote_callback<F>(&mut self, callback: F)
    where
        F: Fn(PromotionReason) -> VmmResult<Vec<u8>> + Send + Sync + 'static,
    {
        self.promote_callback = Some(Box::new(callback));
    }

    /// Load and cache a module
    pub fn load_module(&mut self, id: &str, bytecode: &[u8]) -> VmmResult<Arc<WasmModule>> {
        if let Some(module) = self.module_cache.get(id) {
            return Ok(Arc::clone(module));
        }

        let module = Arc::new(WasmModule::compile(id, bytecode)?);
        self.module_cache.insert(id.to_string(), Arc::clone(&module));
        Ok(module)
    }

    /// Execute a function, trying WASM first
    pub fn execute(
        &self,
        module: &Arc<WasmModule>,
        function: &str,
        args: Vec<WasmValue>,
    ) -> VmmResult<Vec<u8>> {
        self.stats.total_executions.fetch_add(1, Ordering::Relaxed);

        // Try WASM sandbox first
        let sandbox = WasmSandbox::new(Arc::clone(module), self.sandbox_config.clone())?;
        let result = sandbox.execute(function, args);

        match result {
            WasmExecutionResult::Success(output) => {
                self.stats.wasm_successes.fetch_add(1, Ordering::Relaxed);
                self.stats.total_time_us.fetch_add(
                    output.duration.as_micros() as u64,
                    Ordering::Relaxed,
                );
                Ok(output.result)
            }
            WasmExecutionResult::NeedsPromotion(reason) => {
                self.stats.promotions.fetch_add(1, Ordering::Relaxed);

                // Promote to microVM
                if let Some(ref callback) = self.promote_callback {
                    callback(reason)
                } else {
                    Err(VmmError::WasmError(format!(
                        "Promotion required but no callback set: {:?}",
                        reason
                    )))
                }
            }
            WasmExecutionResult::Error(e) => {
                self.stats.errors.fetch_add(1, Ordering::Relaxed);
                Err(VmmError::WasmError(format!("{:?}", e)))
            }
        }
    }

    /// Get execution statistics
    pub fn stats(&self) -> &WasmStats {
        &self.stats
    }

    /// Calculate WASM success rate
    pub fn wasm_success_rate(&self) -> f64 {
        let total = self.stats.total_executions.load(Ordering::Relaxed);
        if total == 0 {
            return 100.0;
        }
        let successes = self.stats.wasm_successes.load(Ordering::Relaxed);
        (successes as f64 / total as f64) * 100.0
    }
}

/// Module compatibility analyzer
pub struct CompatibilityAnalyzer;

impl CompatibilityAnalyzer {
    /// Analyze if a module can run in WASM sandbox
    pub fn analyze(bytecode: &[u8]) -> CompatibilityReport {
        let mut report = CompatibilityReport::default();

        // Check WASM validity
        if bytecode.len() < 8 || &bytecode[0..4] != b"\0asm" {
            report.is_valid_wasm = false;
            report.issues.push("Invalid WASM magic bytes".to_string());
            return report;
        }

        report.is_valid_wasm = true;

        // Check for syscall imports that would require VM
        // In real impl, parse the import section
        let imports = Self::detect_imports(bytecode);

        for import in imports {
            match import.as_str() {
                "wasi_snapshot_preview1" => {
                    report.requires_wasi = true;
                }
                "env" => {
                    // Check specific env imports
                }
                _ if import.starts_with("wasi") => {
                    report.requires_wasi = true;
                }
                _ => {}
            }
        }

        // Determine sandbox compatibility
        report.sandbox_compatible = !report.requires_raw_syscalls && !report.requires_network;

        report
    }

    /// Detect imported modules (simplified)
    fn detect_imports(_bytecode: &[u8]) -> Vec<String> {
        // Real implementation would parse WASM binary
        vec![]
    }
}

/// Compatibility analysis report
#[derive(Debug, Default)]
pub struct CompatibilityReport {
    /// Is valid WASM binary
    pub is_valid_wasm: bool,
    /// Can run in WASM sandbox
    pub sandbox_compatible: bool,
    /// Requires WASI
    pub requires_wasi: bool,
    /// Requires network access
    pub requires_network: bool,
    /// Requires raw syscalls
    pub requires_raw_syscalls: bool,
    /// Detected issues
    pub issues: Vec<String>,
    /// Recommendations
    pub recommendations: Vec<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    // Valid minimal WASM module (empty)
    const MINIMAL_WASM: &[u8] = &[
        0x00, 0x61, 0x73, 0x6d, // Magic: \0asm
        0x01, 0x00, 0x00, 0x00, // Version: 1
    ];

    #[test]
    fn test_wasm_module_compile() {
        let result = WasmModule::compile("test", MINIMAL_WASM);
        assert!(result.is_ok());

        let module = result.unwrap();
        assert_eq!(module.id, "test");
        assert!(module.sandbox_compatible);
    }

    #[test]
    fn test_invalid_wasm() {
        let result = WasmModule::compile("invalid", b"not wasm");
        assert!(result.is_err());
    }

    #[test]
    fn test_sandbox_creation() {
        let module = Arc::new(WasmModule::compile("test", MINIMAL_WASM).unwrap());
        let sandbox = WasmSandbox::new(module, WasmSandboxConfig::default());
        assert!(sandbox.is_ok());
    }

    #[test]
    fn test_compatibility_analyzer() {
        let report = CompatibilityAnalyzer::analyze(MINIMAL_WASM);
        assert!(report.is_valid_wasm);
        assert!(report.sandbox_compatible);
    }

    #[test]
    fn test_hybrid_executor() {
        let mut executor = HybridExecutor::new(WasmSandboxConfig::default());
        let module = executor.load_module("test", MINIMAL_WASM);
        assert!(module.is_ok());

        assert_eq!(executor.wasm_success_rate(), 100.0);
    }

    #[test]
    fn test_wasi_capabilities() {
        let none = WasiCapabilities::none();
        assert!(!none.requires_vm());

        let full = WasiCapabilities::full();
        assert!(full.requires_vm());
    }
}
