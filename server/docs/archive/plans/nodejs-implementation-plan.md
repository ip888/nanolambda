# Node.js Runtime Implementation Plan

## Overview

Implement a Node.js runtime that leverages the Runtime trait architecture, providing JavaScript/TypeScript function execution with the same performance characteristics as the Python runtime (<5ms warm starts, process pooling, real metrics).

## Goals

1. **Runtime Trait Implementation**: Implement the `Runtime` trait for Node.js
2. **Process Management**: Node.js process spawning and lifecycle management
3. **IPC Communication**: stdin/stdout JSON-based IPC similar to Python
4. **Module Support**: Both ES modules and CommonJS
5. **Process Pooling**: Leverage existing pool infrastructure
6. **Metrics Integration**: Use ProcessMetrics for real memory/CPU tracking
7. **Error Handling**: Comprehensive error handling and validation

## Architecture

```
NodeJSRuntime
├── NodeJSExecutor (implements Runtime trait)
│   ├── node_path: PathBuf
│   ├── node_version: String
│   ├── pool: Option<ProcessPool<NodeProcess>>
│   └── enable_warm_starts: bool
├── NodeProcess (implements LanguageProcess concept)
│   ├── child: Child
│   ├── stdin: ChildStdin
│   ├── stdout: BufReader<ChildStdout>
│   ├── code_hash: String
│   ├── stats: ProcessStats
│   └── metrics: Option<ProcessMetrics>
└── Node.js wrapper script
    ├── Loads function code
    ├── Listens on stdin
    ├── Executes handler
    └── Returns results on stdout
```

## Implementation Phases

### Phase 1: Node.js Detection and Setup ✅ (Start Here)

Create basic Node.js runtime structure:

```rust
// crates/runtime/src/nodejs/mod.rs
pub mod executor;
pub mod process;

pub use executor::NodeJSExecutor;
pub use process::NodeProcess;
```

Detect Node.js installation:
- Check for `node` binary in PATH
- Verify version (prefer >= 18.x LTS)
- Fallback to `nodejs` on some systems

### Phase 2: Node.js Process Wrapper Script

Create a Node.js runner script similar to Python's:

```javascript
// Embedded in Rust as string literal
const readline = require('readline');

// Load function code
const functionCode = `${USER_FUNCTION_CODE}`;
const handler = eval(functionCode);

// Create readline interface
const rl = readline.createInterface({
    input: process.stdin,
    output: process.stdout
});

// Process requests
rl.on('line', async (line) => {
    try {
        const request = JSON.parse(line);
        const start = Date.now();
        
        const result = await handler(request.event, request.context);
        const executionMs = Date.now() - start;
        
        const response = {
            success: true,
            result: JSON.stringify(result),
            execution_ms: executionMs
        };
        
        console.log(JSON.stringify(response));
    } catch (error) {
        const response = {
            success: false,
            error: error.message,
            stack: error.stack
        };
        
        console.log(JSON.stringify(response));
    }
});
```

### Phase 3: NodeProcess Implementation

Similar to PythonProcess but for Node.js:

```rust
pub struct NodeProcess {
    child: Child,
    stdin: ChildStdin,
    stdout: BufReader<ChildStdout>,
    code_hash: String,
    stats: ProcessStats,
    created_at: Instant,
    metrics: Option<ProcessMetrics>,
    last_metrics: Option<ProcessMetrics>,
}

impl NodeProcess {
    pub fn new(node_path: &str, function_code: &str) -> Result<Self> {
        // Create wrapper script
        // Spawn node process
        // Setup stdin/stdout pipes
        // Initialize metrics
    }
    
    pub fn invoke(&mut self, event: &Value) -> Result<InvocationResult> {
        // Update metrics before
        // Send JSON request via stdin
        // Read JSON response from stdout
        // Update metrics after
        // Return result
    }
    
    pub fn is_healthy(&mut self) -> bool {
        // Check if process is still running
    }
    
    // ... other methods similar to PythonProcess
}
```

### Phase 4: NodeJSExecutor with Runtime Trait

```rust
pub struct NodeJSExecutor {
    node_path: PathBuf,
    node_version: String,
    base_dir: PathBuf,
    pool: Option<ProcessPool<NodeProcess>>,
    enable_warm_starts: bool,
}

#[async_trait]
impl Runtime for NodeJSExecutor {
    async fn execute(
        &self,
        config: &GenericFunctionConfig,
        event: Value,
    ) -> Result<ExecutionResult, ExecutorError> {
        // Warm start path (if enabled)
        if self.enable_warm_starts && self.pool.is_some() {
            // Use pool
        } else {
            // Cold start path
        }
    }
    
    fn runtime_info(&self) -> RuntimeInfo {
        RuntimeInfo {
            language: Language::NodeJS,
            version: self.node_version.clone(),
            interpreter_path: self.node_path.clone(),
            capabilities: RuntimeCapabilities {
                warm_starts: true,
                async_execution: true,
                streaming: false,
                max_memory_mb: None,
                max_timeout_seconds: None,
            },
        }
    }
    
    fn health_check(&self) -> Result<(), ExecutorError> {
        // Check if node binary exists and is executable
    }
    
    fn set_warm_starts(&mut self, enabled: bool) {
        self.enable_warm_starts = enabled;
    }
    
    fn warm_starts_enabled(&self) -> bool {
        self.enable_warm_starts
    }
}
```

### Phase 5: Testing

Comprehensive test suite:

1. **Unit Tests**
   - Node.js detection
   - Process spawning
   - IPC communication
   - Error handling

2. **Integration Tests**
   - Simple function execution
   - Async handler support
   - Error handling
   - Timeout handling
   - Warm vs cold starts

3. **Example Programs**
   - Hello World
   - JSON processing
   - Async operations
   - Error scenarios

## Node.js Wrapper Script Design

### CommonJS Support

```javascript
// For CommonJS modules
const handler = require('./function.js').handler;
```

### ES Modules Support

```javascript
// For ES modules
import { handler } from './function.js';
```

### Inline Code Support

```javascript
// For inline code (eval approach)
const functionCode = `
exports.handler = async (event, context) => {
    return { message: 'Hello from Node.js!' };
};
`;

const handlerModule = {};
eval(`(function(exports, require, module, __filename, __dirname) {
    ${functionCode}
})`)(handlerModule, require, {exports: handlerModule}, '', '');

const handler = handlerModule.handler;
```

## IPC Protocol

Same JSON protocol as Python:

### Request Format
```json
{
    "event": { "name": "test" },
    "context": {}
}
```

### Success Response
```json
{
    "success": true,
    "result": "{\"message\": \"success\"}",
    "execution_ms": 15
}
```

### Error Response
```json
{
    "success": false,
    "error": "ReferenceError: foo is not defined",
    "stack": "ReferenceError: foo is not defined\n    at handler...",
    "execution_ms": 5
}
```

## File Structure

```
crates/runtime/src/
├── nodejs/
│   ├── mod.rs           # Module exports
│   ├── executor.rs      # NodeJSExecutor + Runtime impl
│   ├── process.rs       # NodeProcess implementation
│   └── wrapper.js       # Node.js wrapper script template
└── lib.rs               # Add nodejs module
```

## Node.js Version Support

Target versions:
- **Primary**: Node.js 18.x LTS (until April 2025)
- **Support**: Node.js 20.x LTS (until April 2026)
- **Future**: Node.js 22.x LTS (October 2024+)

Minimum: Node.js 18.0.0 (for native fetch, test runner, etc.)

## Dependencies

**No new Rust dependencies needed!** ✅

Everything uses existing infrastructure:
- Process spawning: `std::process::Command`
- IPC: stdin/stdout with `serde_json`
- Metrics: existing `ProcessMetrics`
- Pooling: existing `ProcessPool` pattern

## Error Handling

### Node.js-Specific Errors

```rust
#[derive(Error, Debug)]
pub enum NodeError {
    #[error("Node.js not found. Please install Node.js 18.x or later")]
    NodeNotFound,
    
    #[error("Node.js version {0} is not supported. Minimum version is 18.0.0")]
    UnsupportedVersion(String),
    
    #[error("Failed to spawn Node.js process: {0}")]
    SpawnFailed(String),
    
    #[error("Invalid JavaScript code: {0}")]
    InvalidCode(String),
    
    #[error("Handler function not found or not exported")]
    HandlerNotFound,
}
```

## Performance Goals

Match Python runtime performance:

| Metric | Target | Notes |
|--------|--------|-------|
| Cold Start | <50ms | First invocation |
| Warm Start | <5ms | Subsequent invocations |
| Memory Overhead | <30MB | Base Node.js process |
| Pool Creation | <100ms | Spawn + initialize |
| IPC Latency | <1ms | JSON serialization + pipe |

## Testing Strategy

### Unit Tests (10+ tests)
- `test_node_detection` - Find node binary
- `test_node_version_parsing` - Parse version string
- `test_process_spawn` - Spawn node process
- `test_process_invoke` - Single invocation
- `test_process_health` - Health check
- `test_error_handling` - JavaScript errors
- `test_async_handler` - Async function support
- `test_json_serialization` - Complex JSON objects
- `test_timeout` - Timeout handling
- `test_metrics_collection` - Memory/CPU tracking

### Integration Tests (5+ tests)
- `test_hello_world` - Simple function
- `test_async_operations` - Async/await
- `test_warm_starts` - Process reuse
- `test_multiple_invocations` - Series of calls
- `test_error_recovery` - Error handling

### Example Programs
- `examples/nodejs_hello_world.rs`
- `examples/nodejs_async_demo.rs`
- `examples/nodejs_vs_python.rs`

## Implementation Checklist

### Phase 1: Foundation
- [ ] Create `crates/runtime/src/nodejs/` directory
- [ ] Create `mod.rs` with module structure
- [ ] Implement Node.js detection
- [ ] Add version checking

### Phase 2: Process Management
- [ ] Design Node.js wrapper script
- [ ] Implement `NodeProcess` struct
- [ ] Add process spawning
- [ ] Add IPC communication
- [ ] Integrate metrics

### Phase 3: Executor
- [ ] Implement `NodeJSExecutor` struct
- [ ] Implement `Runtime` trait
- [ ] Add process pooling
- [ ] Add cold start path
- [ ] Add warm start path

### Phase 4: Testing
- [ ] Write unit tests
- [ ] Write integration tests
- [ ] Create example programs
- [ ] Performance benchmarks

### Phase 5: Documentation
- [ ] API documentation
- [ ] Usage examples
- [ ] Migration guide (Python → Node.js)
- [ ] Performance comparison

## Success Criteria

✅ Node.js functions execute correctly
✅ Runtime trait fully implemented
✅ Warm starts <5ms
✅ Cold starts <50ms
✅ All tests passing
✅ Real metrics collection
✅ Async handler support
✅ Error handling comprehensive
✅ Documentation complete

## Example Usage (Goal)

```rust
use nanolambda_runtime::{NodeJSExecutor, Runtime, GenericFunctionConfig, Language};

#[tokio::main]
async fn main() -> Result<()> {
    let mut runtime = NodeJSExecutor::new()?;
    runtime.set_warm_starts(true);
    
    let config = GenericFunctionConfig::new(
        "my-function".to_string(),
        Language::NodeJS,
        r#"
        exports.handler = async (event) => {
            return { message: `Hello, ${event.name}!` };
        };
        "#.to_string(),
        "handler".to_string(),
    );
    
    let event = json!({ "name": "World" });
    let result = runtime.execute(&config, event).await?;
    
    println!("Result: {:?}", result);
    Ok(())
}
```

## Next Steps

1. ✅ **Create implementation plan** - Complete
2. ⏭️ **Implement Node.js detection** - Detect node binary, check version
3. ⏭️ **Create NodeProcess** - Process spawning and IPC
4. ⏭️ **Implement NodeJSExecutor** - Runtime trait implementation
5. ⏭️ **Add tests** - Comprehensive test coverage
6. ⏭️ **Create examples** - Working demonstrations
7. ⏭️ **Document** - Complete documentation

Let's start with Phase 1: Node.js detection and basic structure!
