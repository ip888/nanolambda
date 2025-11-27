# Generic Runtime Trait Design

## Overview

Design a generic `Runtime` trait that provides a consistent interface for executing functions across multiple languages (Python, Node.js, Java, etc.) while leveraging shared infrastructure like process pooling, metrics, and resource management.

## Goals

1. **Language Abstraction**: Support multiple runtime languages with a unified API
2. **Code Reuse**: Share process pooling, metrics, and resource management logic
3. **Type Safety**: Leverage Rust's type system for compile-time guarantees
4. **Extensibility**: Easy to add new language runtimes
5. **Performance**: Zero-cost abstractions where possible

## Current State

### Existing Architecture

```
PythonExecutor (concrete)
├── ProcessPool (concrete)
│   └── WarmProcess (concrete, Python-specific)
├── execute() method
└── Metrics collection
```

**Problems:**
- Python-specific implementation
- Difficult to add Node.js or Java
- Code duplication inevitable
- No shared interface

## Proposed Architecture

### Core Trait Hierarchy

```rust
/// Generic runtime interface for any language
pub trait Runtime: Send + Sync {
    /// Execute a function with the given configuration and event
    async fn execute(
        &self,
        config: &FunctionConfig,
        event: serde_json::Value,
    ) -> Result<ExecutionResult, RuntimeError>;
    
    /// Get runtime information (name, version)
    fn runtime_info(&self) -> RuntimeInfo;
    
    /// Check if the runtime is healthy and available
    fn health_check(&self) -> Result<(), RuntimeError>;
    
    /// Enable/disable warm starts
    fn set_warm_starts(&mut self, enabled: bool);
}

/// Language-specific executor trait (implemented by each language)
pub trait LanguageExecutor: Send + Sync {
    /// Spawn a new process for this language
    fn spawn_process(
        &self,
        code: &str,
        config: &FunctionConfig,
    ) -> Result<Box<dyn LanguageProcess>, RuntimeError>;
    
    /// Get the language name
    fn language(&self) -> Language;
    
    /// Validate function code
    fn validate_code(&self, code: &str) -> Result<(), RuntimeError>;
}

/// Process interface (wraps Child process with language-specific logic)
pub trait LanguageProcess: Send {
    /// Invoke the function with an event
    fn invoke(&mut self, event: &serde_json::Value) -> Result<InvocationResult, RuntimeError>;
    
    /// Check if process is healthy
    fn is_healthy(&self) -> bool;
    
    /// Get process ID
    fn pid(&self) -> u32;
    
    /// Get process statistics
    fn stats(&self) -> &ProcessStats;
    
    /// Update metrics
    fn update_metrics(&mut self) -> Result<(), RuntimeError>;
    
    /// Get current memory in MB
    fn memory_mb(&self) -> u64;
    
    /// Get peak memory in MB
    fn peak_memory_mb(&self) -> u64;
    
    /// Get CPU percentage
    fn cpu_percent(&self) -> f64;
}
```

### Structure Diagram

```
┌─────────────────────────────────────────────────────┐
│                   Runtime Trait                     │
│  (User-facing API - async fn execute)               │
└────────────────┬────────────────────────────────────┘
                 │
                 │ Uses
                 ▼
┌─────────────────────────────────────────────────────┐
│              GenericRuntime<E>                      │
│  (Generic implementation with process pooling)      │
│  E: LanguageExecutor                                │
└────────────────┬────────────────────────────────────┘
                 │
                 │ Contains
                 ▼
┌─────────────────────────────────────────────────────┐
│          ProcessPool<P: LanguageProcess>            │
│  (Generic pool, language-agnostic)                  │
└────────────────┬────────────────────────────────────┘
                 │
                 │ Stores
                 ▼
┌──────────────┬──────────────┬──────────────────────┐
│ PythonProcess│ NodeProcess  │ JavaProcess          │
│ (Python)     │ (Node.js)    │ (Java)               │
│              │              │                      │
│ impl         │ impl         │ impl                 │
│ LanguageProc │ LanguageProc │ LanguageProcess      │
└──────────────┴──────────────┴──────────────────────┘
```

## Implementation Strategy

### Phase 1: Extract Common Types

Create language-agnostic types:

```rust
// crates/runtime/src/types.rs

/// Language enum
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Language {
    Python,
    NodeJS,
    Java,
    // Future: Go, Ruby, etc.
}

/// Runtime information
#[derive(Debug, Clone)]
pub struct RuntimeInfo {
    pub language: Language,
    pub version: String,
    pub interpreter_path: PathBuf,
}

/// Invocation result from language process
#[derive(Debug)]
pub struct InvocationResult {
    pub success: bool,
    pub output: String,
    pub error: Option<String>,
    pub execution_ms: u64,
}

/// Generic function configuration
#[derive(Debug, Clone)]
pub struct FunctionConfig {
    pub name: String,
    pub language: Language,
    pub code: String,
    pub handler: String,
    pub environment: HashMap<String, String>,
    pub memory_limit_mb: u64,
    pub timeout_seconds: u64,
    pub working_dir: Option<PathBuf>,
}
```

### Phase 2: Create Generic ProcessPool

```rust
// crates/runtime/src/generic_pool.rs

pub struct ProcessPool<P: LanguageProcess> {
    processes: Arc<Mutex<HashMap<String, P>>>,
    max_size: usize,
    max_age_seconds: u64,
    max_invocations: u64,
}

impl<P: LanguageProcess> ProcessPool<P> {
    pub fn execute_warm(
        &self,
        function_name: &str,
        process_factory: impl Fn() -> Result<P, RuntimeError>,
        event: &Value,
    ) -> Result<(InvocationResult, ProcessMetrics), RuntimeError> {
        // Get or create process
        // Invoke function
        // Update metrics
        // Return results
    }
}
```

### Phase 3: Implement LanguageProcess for Python

```rust
// crates/runtime/src/python/process.rs

pub struct PythonProcess {
    child: Child,
    stdin: ChildStdin,
    stdout: BufReader<ChildStdout>,
    code_hash: String,
    stats: ProcessStats,
    metrics: Option<ProcessMetrics>,
    last_metrics: Option<ProcessMetrics>,
}

impl LanguageProcess for PythonProcess {
    fn invoke(&mut self, event: &Value) -> Result<InvocationResult, RuntimeError> {
        // Current implementation logic
    }
    
    fn is_healthy(&self) -> bool { /* ... */ }
    fn pid(&self) -> u32 { self.child.id() }
    fn stats(&self) -> &ProcessStats { &self.stats }
    // ... other methods
}
```

### Phase 4: Implement LanguageExecutor for Python

```rust
// crates/runtime/src/python/executor.rs

pub struct PythonExecutor {
    python_path: PathBuf,
    python_version: String,
    base_dir: PathBuf,
}

impl LanguageExecutor for PythonExecutor {
    fn spawn_process(
        &self,
        code: &str,
        config: &FunctionConfig,
    ) -> Result<Box<dyn LanguageProcess>, RuntimeError> {
        let process = PythonProcess::new(&self.python_path, code)?;
        Ok(Box::new(process))
    }
    
    fn language(&self) -> Language {
        Language::Python
    }
    
    fn validate_code(&self, code: &str) -> Result<(), RuntimeError> {
        // Python syntax validation
    }
}
```

### Phase 5: Create GenericRuntime

```rust
// crates/runtime/src/generic_runtime.rs

pub struct GenericRuntime<E: LanguageExecutor> {
    executor: E,
    pool: Option<ProcessPool<Box<dyn LanguageProcess>>>,
    enable_warm_starts: bool,
}

impl<E: LanguageExecutor> Runtime for GenericRuntime<E> {
    async fn execute(
        &self,
        config: &FunctionConfig,
        event: Value,
    ) -> Result<ExecutionResult, RuntimeError> {
        if self.enable_warm_starts && self.pool.is_some() {
            // Warm path
            let pool = self.pool.as_ref().unwrap();
            let factory = || self.executor.spawn_process(&config.code, config);
            let (result, metrics) = pool.execute_warm(&config.name, factory, &event)?;
            
            Ok(ExecutionResult {
                success: result.success,
                result: Some(result.output),
                error: result.error,
                metrics: self.build_metrics(metrics, result.execution_ms),
            })
        } else {
            // Cold path
            self.cold_start_execution(config, event)
        }
    }
    
    fn runtime_info(&self) -> RuntimeInfo { /* ... */ }
    fn health_check(&self) -> Result<(), RuntimeError> { /* ... */ }
    fn set_warm_starts(&mut self, enabled: bool) { /* ... */ }
}
```

## Usage Examples

### Python Runtime

```rust
use nanolambda_runtime::{GenericRuntime, PythonExecutor, Runtime, FunctionConfig, Language};

let executor = PythonExecutor::new()?;
let mut runtime = GenericRuntime::new(executor);
runtime.set_warm_starts(true);

let config = FunctionConfig {
    name: "my-function".to_string(),
    language: Language::Python,
    code: "def handler(event, context): return {'hello': 'world'}".to_string(),
    handler: "handler".to_string(),
    // ...
};

let result = runtime.execute(&config, json!({})).await?;
```

### Node.js Runtime (Future)

```rust
let executor = NodeJSExecutor::new()?;
let mut runtime = GenericRuntime::new(executor);
runtime.set_warm_starts(true);

let config = FunctionConfig {
    name: "my-function".to_string(),
    language: Language::NodeJS,
    code: "exports.handler = async (event) => ({ hello: 'world' })".to_string(),
    handler: "handler".to_string(),
    // ...
};

let result = runtime.execute(&config, json!({})).await?;
```

### Multi-Runtime System

```rust
use std::collections::HashMap;

pub struct RuntimeManager {
    runtimes: HashMap<Language, Box<dyn Runtime>>,
}

impl RuntimeManager {
    pub fn new() -> Self {
        let mut runtimes: HashMap<Language, Box<dyn Runtime>> = HashMap::new();
        
        // Register Python
        if let Ok(py_exec) = PythonExecutor::new() {
            runtimes.insert(
                Language::Python,
                Box::new(GenericRuntime::new(py_exec)),
            );
        }
        
        // Register Node.js
        if let Ok(node_exec) = NodeJSExecutor::new() {
            runtimes.insert(
                Language::NodeJS,
                Box::new(GenericRuntime::new(node_exec)),
            );
        }
        
        Self { runtimes }
    }
    
    pub async fn execute(
        &self,
        config: &FunctionConfig,
        event: Value,
    ) -> Result<ExecutionResult, RuntimeError> {
        let runtime = self.runtimes
            .get(&config.language)
            .ok_or(RuntimeError::UnsupportedLanguage(config.language))?;
        
        runtime.execute(config, event).await
    }
}
```

## Benefits

### 1. Code Reuse
- Single `ProcessPool` implementation
- Shared metrics collection
- Common resource management
- Unified error handling

### 2. Type Safety
- Compile-time language validation
- Type-safe process handling
- Trait bounds enforce contracts

### 3. Easy Extension
Adding a new language requires:
1. Implement `LanguageProcess` (spawn, invoke, health)
2. Implement `LanguageExecutor` (validate, spawn)
3. Create `GenericRuntime<YourExecutor>`

### 4. Testing
- Mock implementations for testing
- Language-agnostic integration tests
- Easier to test pool behavior

## Migration Path

### Step 1: Create new trait module (no breaking changes)
```rust
// crates/runtime/src/trait.rs - NEW
pub trait Runtime { /* ... */ }
pub trait LanguageExecutor { /* ... */ }
pub trait LanguageProcess { /* ... */ }
```

### Step 2: Refactor Python implementation (internal change)
```rust
// crates/runtime/src/python/process.rs - NEW
impl LanguageProcess for PythonProcess { /* ... */ }

// crates/runtime/src/python/executor.rs - REFACTOR
impl LanguageExecutor for PythonExecutor { /* ... */ }
```

### Step 3: Create GenericRuntime (new API, keeps old API)
```rust
// crates/runtime/src/generic_runtime.rs - NEW
pub struct GenericRuntime<E: LanguageExecutor> { /* ... */ }

// Old API still works:
let executor = PythonExecutor::new()?;
executor.execute(config, event)?; // Still works!

// New API available:
let runtime = GenericRuntime::new(PythonExecutor::new()?);
runtime.execute(&config, event).await?;
```

### Step 4: Gradually migrate users to new API
- Update examples
- Update documentation
- Deprecate old API (but keep working)

## File Structure

```
crates/runtime/src/
├── lib.rs
├── types.rs              # Common types (Language, RuntimeInfo, etc.)
├── error.rs              # RuntimeError enum
├── runtime_trait.rs      # Runtime trait definition
├── executor_trait.rs     # LanguageExecutor trait
├── process_trait.rs      # LanguageProcess trait
├── generic_runtime.rs    # GenericRuntime<E> implementation
├── generic_pool.rs       # ProcessPool<P> implementation
├── metrics.rs            # ProcessMetrics (existing)
├── python/
│   ├── mod.rs
│   ├── executor.rs       # PythonExecutor (refactored)
│   ├── process.rs        # PythonProcess (implements LanguageProcess)
│   └── validator.rs      # Python code validation
├── nodejs/
│   ├── mod.rs
│   ├── executor.rs       # NodeJSExecutor (new)
│   ├── process.rs        # NodeProcess (new)
│   └── validator.rs      # JavaScript validation
└── java/
    ├── mod.rs
    ├── executor.rs       # JavaExecutor (future)
    └── process.rs        # JavaProcess (future)
```

## Performance Considerations

### Zero-Cost Abstractions
- Traits compile to static dispatch when type is known
- No runtime overhead for trait calls
- Inlining optimizations apply

### Dynamic Dispatch (when needed)
```rust
// Static dispatch (zero cost)
let runtime = GenericRuntime::new(PythonExecutor::new()?);
runtime.execute(&config, event).await?; // Optimized away!

// Dynamic dispatch (when needed for multi-language)
let runtime: Box<dyn Runtime> = Box::new(GenericRuntime::new(executor));
runtime.execute(&config, event).await?; // Small vtable overhead
```

### Benchmark Goals
- Warm start: <5ms (same as current)
- Cold start: <50ms (same as current)
- Trait overhead: <0.1ms (negligible)

## Testing Strategy

### Unit Tests
```rust
#[cfg(test)]
mod tests {
    use super::*;
    
    struct MockExecutor;
    impl LanguageExecutor for MockExecutor {
        // Mock implementation
    }
    
    #[test]
    fn test_generic_runtime_with_mock() {
        let runtime = GenericRuntime::new(MockExecutor);
        // Test runtime behavior without actual processes
    }
}
```

### Integration Tests
```rust
#[tokio::test]
async fn test_python_runtime() {
    let executor = PythonExecutor::new().unwrap();
    let runtime = GenericRuntime::new(executor);
    
    let config = FunctionConfig { /* ... */ };
    let result = runtime.execute(&config, json!({})).await.unwrap();
    
    assert!(result.success);
}
```

## Next Steps

1. ✅ **Design document** - Complete
2. ⏭️ **Implement trait definitions** - Define Runtime, LanguageExecutor, LanguageProcess
3. ⏭️ **Refactor Python to use traits** - Make PythonProcess implement LanguageProcess
4. ⏭️ **Create GenericRuntime** - Implement generic runtime with pooling
5. ⏭️ **Test migration** - Ensure backward compatibility
6. ⏭️ **Update documentation** - Document new trait-based API
7. ⏭️ **Prepare for Node.js** - Structure ready for new language

## Summary

This design provides:
- ✅ Clean abstraction for multiple languages
- ✅ Type-safe, compile-time validation
- ✅ Code reuse (pool, metrics, resource management)
- ✅ Easy extensibility
- ✅ Backward compatibility
- ✅ Zero-cost abstractions
- ✅ Clear migration path

The trait-based architecture positions NanoLambda to support Python, Node.js, Java, and future languages with minimal code duplication and maximum type safety.
