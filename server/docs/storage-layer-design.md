# Storage Layer Design

## Overview

The storage layer provides persistent storage for Lambda functions, their configurations, and execution history. This replaces the current hardcoded function handling with a proper database-backed system.

## Architecture Choice: SQLite

**Why SQLite?**
- ✅ Zero configuration (embedded database)
- ✅ ACID transactions
- ✅ Fast for local deployments
- ✅ Easy backup (single file)
- ✅ Sufficient for single-node deployments
- ✅ Migration path to PostgreSQL for distributed

## Schema Design

### Functions Table

```sql
CREATE TABLE functions (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    name TEXT NOT NULL UNIQUE,
    runtime TEXT NOT NULL,          -- 'python3.11', 'nodejs20', etc.
    handler TEXT NOT NULL,           -- 'index.handler', 'main.handler'
    code TEXT NOT NULL,              -- Base64-encoded source code
    code_hash TEXT NOT NULL,         -- SHA256 for cache validation
    memory_mb INTEGER NOT NULL,
    timeout_ms INTEGER NOT NULL,
    environment TEXT,                -- JSON-encoded env vars
    created_at INTEGER NOT NULL,     -- Unix timestamp
    updated_at INTEGER NOT NULL,
    last_invoked_at INTEGER,
    invocation_count INTEGER DEFAULT 0,
    total_execution_time_ms INTEGER DEFAULT 0,
    status TEXT DEFAULT 'active'     -- 'active', 'disabled', 'deleted'
);

CREATE INDEX idx_functions_name ON functions(name);
CREATE INDEX idx_functions_status ON functions(status);
CREATE INDEX idx_functions_runtime ON functions(runtime);
```

### Invocations Table (for history/metrics)

```sql
CREATE TABLE invocations (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    function_id INTEGER NOT NULL,
    request_id TEXT NOT NULL UNIQUE,
    status TEXT NOT NULL,            -- 'success', 'error', 'timeout'
    started_at INTEGER NOT NULL,
    completed_at INTEGER,
    execution_time_ms INTEGER,
    memory_used_mb INTEGER,
    cold_start BOOLEAN DEFAULT 0,
    error_message TEXT,
    FOREIGN KEY (function_id) REFERENCES functions(id)
);

CREATE INDEX idx_invocations_function_id ON invocations(function_id);
CREATE INDEX idx_invocations_started_at ON invocations(started_at);
CREATE INDEX idx_invocations_status ON invocations(status);
```

### Tags Table (optional, for organization)

```sql
CREATE TABLE tags (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    function_id INTEGER NOT NULL,
    key TEXT NOT NULL,
    value TEXT NOT NULL,
    FOREIGN KEY (function_id) REFERENCES functions(id)
);

CREATE INDEX idx_tags_function_id ON tags(function_id);
CREATE INDEX idx_tags_key ON tags(key);
```

## API Interface

```rust
pub struct StorageManager {
    db: Arc<Mutex<Connection>>,
    db_path: PathBuf,
}

impl StorageManager {
    // Lifecycle
    pub fn new(db_path: impl AsRef<Path>) -> Result<Self>;
    pub fn init_schema(&self) -> Result<()>;
    
    // Function CRUD
    pub fn create_function(&self, config: FunctionConfig) -> Result<i64>;
    pub fn get_function(&self, name: &str) -> Result<Option<Function>>;
    pub fn update_function(&self, name: &str, config: FunctionConfig) -> Result<()>;
    pub fn delete_function(&self, name: &str) -> Result<()>;
    pub fn list_functions(&self) -> Result<Vec<Function>>;
    
    // Invocation tracking
    pub fn record_invocation(&self, record: InvocationRecord) -> Result<i64>;
    pub fn get_invocation_history(&self, function_name: &str, limit: usize) -> Result<Vec<InvocationRecord>>;
    
    // Metrics
    pub fn get_function_stats(&self, name: &str) -> Result<FunctionStats>;
    pub fn update_invocation_metrics(&self, function_name: &str, execution_time: u64, cold_start: bool) -> Result<()>;
}
```

## Data Models

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FunctionConfig {
    pub name: String,
    pub runtime: String,
    pub handler: String,
    pub code: String,                    // Base64 or raw
    pub memory_mb: u64,
    pub timeout_ms: u64,
    pub environment: HashMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Function {
    pub id: i64,
    pub name: String,
    pub runtime: String,
    pub handler: String,
    pub code: String,
    pub code_hash: String,
    pub memory_mb: u64,
    pub timeout_ms: u64,
    pub environment: HashMap<String, String>,
    pub created_at: u64,
    pub updated_at: u64,
    pub last_invoked_at: Option<u64>,
    pub invocation_count: u64,
    pub total_execution_time_ms: u64,
    pub status: FunctionStatus,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InvocationRecord {
    pub function_id: i64,
    pub request_id: String,
    pub status: InvocationStatus,
    pub started_at: u64,
    pub completed_at: Option<u64>,
    pub execution_time_ms: Option<u64>,
    pub memory_used_mb: Option<u64>,
    pub cold_start: bool,
    pub error_message: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FunctionStats {
    pub invocation_count: u64,
    pub total_execution_time_ms: u64,
    pub avg_execution_time_ms: f64,
    pub cold_start_count: u64,
    pub error_count: u64,
    pub last_invoked_at: Option<u64>,
}
```

## Integration Points

### 1. API Handlers

Current hardcoded handler:
```rust
// crates/api-server/src/handlers.rs
pub async fn invoke_function(name: String, payload: serde_json::Value) -> Result<InvokeResponse> {
    // Hardcoded: Always runs "print('Hello')"
    let code = "print('Hello')";
    executor.execute(code, payload).await?
}
```

New storage-backed handler:
```rust
pub async fn invoke_function(
    State(state): State<AppState>,
    Path(name): Path<String>,
    Json(payload): Json<serde_json::Value>,
) -> Result<Json<InvokeResponse>, StatusCode> {
    // Load function from storage
    let function = state.storage
        .get_function(&name)
        .await
        .ok_or(StatusCode::NOT_FOUND)?;
    
    // Execute with stored code
    let result = state.executor
        .execute(&function.code, payload)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    
    // Record invocation
    state.storage.record_invocation(InvocationRecord {
        function_id: function.id,
        request_id: Uuid::new_v4().to_string(),
        status: InvocationStatus::Success,
        started_at: SystemTime::now(),
        completed_at: Some(SystemTime::now()),
        execution_time_ms: Some(result.execution_ms),
        memory_used_mb: result.memory_mb,
        cold_start: result.cold_start,
        error_message: None,
    }).await?;
    
    Ok(Json(result))
}
```

### 2. Deployment Endpoint

```rust
pub async fn deploy_function(
    State(state): State<AppState>,
    Json(config): Json<FunctionConfig>,
) -> Result<Json<DeployResponse>, StatusCode> {
    // Compute code hash
    let code_hash = sha256(&config.code);
    
    // Check if function exists
    if let Some(existing) = state.storage.get_function(&config.name).await {
        // Update existing function
        state.storage.update_function(&config.name, config).await?;
    } else {
        // Create new function
        state.storage.create_function(config).await?;
    }
    
    Ok(Json(DeployResponse {
        function_name: config.name,
        version: code_hash,
        status: "deployed".to_string(),
    }))
}
```

## Migration Strategy

### Phase 1: Add Storage (Backward Compatible)
- Implement storage layer
- Keep hardcoded behavior as fallback
- Add feature flag: `use_storage = true/false`

### Phase 2: Use Storage for New Functions
- New deployments go to storage
- Existing behavior unchanged
- Run both systems in parallel

### Phase 3: Full Migration
- Remove hardcoded functions
- Make storage mandatory
- Clean up old code paths

## Performance Considerations

### Query Optimization
- Use prepared statements
- Index on frequently queried columns
- Connection pooling (r2d2 or deadpool)

### Code Storage
- Store code as TEXT (SQLite limit: 1GB per text field)
- Alternatively: Store large code in filesystem, path in DB
- Consider compression for large functions

### Invocation Tracking
- Async writes (don't block response)
- Batch inserts for high throughput
- Optional: Separate read replica for metrics

## Dependencies

```toml
[dependencies]
# SQLite
rusqlite = { version = "0.31", features = ["bundled"] }
# Or use async version:
# sqlx = { version = "0.7", features = ["runtime-tokio-rustls", "sqlite"] }

# Hashing
sha2 = "0.10"

# Timestamps
chrono = "0.4"
```

## Testing Strategy

### Unit Tests
- `test_create_function()`
- `test_get_function_not_found()`
- `test_update_function()`
- `test_delete_function()`
- `test_list_functions()`
- `test_record_invocation()`

### Integration Tests
- `test_storage_with_api_handlers()`
- `test_function_lifecycle()`
- `test_concurrent_invocations_tracking()`
- `test_metrics_accuracy()`

## Next Implementation Steps

1. **Create storage crate skeleton**
   ```bash
   cargo new --lib crates/storage
   ```

2. **Implement StorageManager with rusqlite**

3. **Add migration scripts for schema**

4. **Update API handlers to use storage**

5. **Add tests for CRUD operations**

6. **Test end-to-end deployment and invocation**

7. **Document storage API usage**
