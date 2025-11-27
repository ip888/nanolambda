# Function Versioning & Aliases

**Status**: 📋 Design Proposal  
**Priority**: HIGH  
**Complexity**: Medium  
**Impact**: Solves process pool code update issue + adds enterprise features

---

## Problem Statement

### Current Issues
1. **Code update invalidation**: Updating function code doesn't invalidate process pool
2. **No rollback**: Can't revert to previous working version
3. **No testing**: Can't test new version before switching production traffic
4. **Lost history**: Old versions deleted permanently

### Real-World Scenario
```
1. Function v1 deployed to production (working)
2. Developer updates code to v2
3. v2 has a bug and causes errors
4. Can't rollback - v1 code is lost!
5. Process pool still running v1 anyway (cache issue)
```

---

## Solution: Function Versioning (AWS Lambda Model)

### Concepts

#### 1. Versions (Immutable Snapshots)
- Each publish creates a **new version number**
- Versions are **immutable** - code never changes
- Version numbers: 1, 2, 3, ... (auto-incrementing)
- Special version: `$LATEST` (always points to newest)

#### 2. Aliases (Traffic Routing)
- Named pointers to versions: `production`, `staging`, `dev`
- Can point to multiple versions with traffic weights
- Mutable - can be updated to point to different versions

#### 3. Workflow
```
Code Update → New Version → Test $LATEST → Update Alias → Production Traffic Switched
```

---

## Architecture Design

### Database Schema

```sql
-- Enhanced functions table
CREATE TABLE functions (
    id INTEGER PRIMARY KEY,
    name TEXT NOT NULL,
    version INTEGER NOT NULL DEFAULT 1,
    runtime TEXT NOT NULL,
    handler TEXT NOT NULL,
    code TEXT NOT NULL,
    code_hash TEXT NOT NULL,
    memory_mb INTEGER DEFAULT 128,
    timeout_ms INTEGER DEFAULT 5000,
    environment TEXT DEFAULT '{}',
    status TEXT DEFAULT 'active',
    is_latest BOOLEAN DEFAULT TRUE,
    published_at INTEGER,
    created_at INTEGER NOT NULL,
    updated_at INTEGER NOT NULL,
    UNIQUE(name, version)
);

CREATE INDEX idx_functions_name ON functions(name);
CREATE INDEX idx_functions_name_version ON functions(name, version);
CREATE INDEX idx_functions_name_latest ON functions(name, is_latest) WHERE is_latest = TRUE;

-- Function aliases
CREATE TABLE function_aliases (
    id INTEGER PRIMARY KEY,
    function_name TEXT NOT NULL,
    alias TEXT NOT NULL,
    description TEXT,
    created_at INTEGER NOT NULL,
    updated_at INTEGER NOT NULL,
    UNIQUE(function_name, alias)
);

CREATE INDEX idx_aliases_name ON function_aliases(function_name);

-- Alias version routing (for traffic splitting)
CREATE TABLE alias_routing (
    id INTEGER PRIMARY KEY,
    alias_id INTEGER NOT NULL,
    version INTEGER NOT NULL,
    traffic_weight REAL DEFAULT 1.0,
    FOREIGN KEY (alias_id) REFERENCES function_aliases(id) ON DELETE CASCADE
);
```

### API Endpoints

```rust
// Version Management
POST   /functions                              // Create v1
PUT    /functions/{name}?publish=true          // Create new version
GET    /functions/{name}/versions              // List all versions
GET    /functions/{name}/versions/{version}    // Get specific version
DELETE /functions/{name}/versions/{version}    // Delete old version

// Alias Management
POST   /functions/{name}/aliases               // Create alias
GET    /functions/{name}/aliases               // List aliases
GET    /functions/{name}/aliases/{alias}       // Get alias details
PUT    /functions/{name}/aliases/{alias}       // Update alias routing
DELETE /functions/{name}/aliases/{alias}       // Delete alias

// Invocation (version-aware)
POST   /functions/{name}/invoke                     // Use default alias (or $LATEST if no alias)
POST   /functions/{name}:{version}/invoke           // Invoke specific version
POST   /functions/{name}:{alias}/invoke             // Invoke via alias
POST   /functions/{name}:$LATEST/invoke             // Always newest version
```

---

## Implementation Details

### 1. StorageManager Extensions

```rust
impl StorageManager {
    /// Create function (always starts at version 1)
    pub fn create_function(&self, config: FunctionConfig) -> Result<(i64, i64)> {
        // Returns (function_id, version)
        // Sets is_latest = TRUE
    }

    /// Publish new version
    pub fn publish_version(&self, name: &str, config: FunctionConfig) -> Result<i64> {
        // 1. Get max version for function
        // 2. Create new row with version = max + 1
        // 3. Set old $LATEST to is_latest = FALSE
        // 4. Set new version to is_latest = TRUE
        // 5. Return new version number
    }

    /// Get specific version
    pub fn get_function_version(&self, name: &str, version: i64) -> Result<Option<Function>> {
        // WHERE name = ? AND version = ?
    }

    /// Get $LATEST version
    pub fn get_latest_version(&self, name: &str) -> Result<Option<Function>> {
        // WHERE name = ? AND is_latest = TRUE
    }

    /// List all versions
    pub fn list_versions(&self, name: &str) -> Result<Vec<Function>> {
        // WHERE name = ? ORDER BY version DESC
    }

    /// Delete specific version
    pub fn delete_version(&self, name: &str, version: i64) -> Result<()> {
        // Cannot delete $LATEST
        // Soft delete: status = 'deleted'
    }

    // Alias Management
    pub fn create_alias(&self, name: &str, alias: &str, version: i64) -> Result<i64>;
    pub fn update_alias(&self, name: &str, alias: &str, version: i64) -> Result<()>;
    pub fn update_alias_routing(&self, name: &str, alias: &str, routing: Vec<VersionWeight>) -> Result<()>;
    pub fn get_alias(&self, name: &str, alias: &str) -> Result<Option<AliasInfo>>;
    pub fn resolve_alias(&self, name: &str, alias: &str) -> Result<i64>; // Returns version number
}
```

### 2. Runtime Integration

```rust
impl PythonExecutor {
    pub fn execute(&self, function_id: i64, version: i64, event: Value) -> Result<ExecutionResult> {
        // Use (function_id, version) as cache key instead of just function_id
        let cache_key = format!("{}:{}", function_id, version);
        
        // Check if process exists for this version
        if let Some(process) = self.pool.get(&cache_key) {
            // Use existing process
        } else {
            // Create new process for this version
        }
    }
}
```

### 3. Handler Updates

```rust
// handlers.rs

pub async fn invoke_function(
    Path(name): Path<String>,
    State(state): State<AppState>,
    Json(input): Json<Value>,
) -> Result<Json<InvocationResponse>, ApiError> {
    // Parse name for version/alias
    let (function_name, target) = parse_function_target(&name)?;
    // Examples:
    // "my-func" → ("my-func", Target::Latest)
    // "my-func:3" → ("my-func", Target::Version(3))
    // "my-func:production" → ("my-func", Target::Alias("production"))
    // "my-func:$LATEST" → ("my-func", Target::Latest)
    
    // Resolve to specific version
    let version = match target {
        Target::Version(v) => v,
        Target::Alias(alias) => {
            state.storage.resolve_alias(&function_name, &alias)?
        },
        Target::Latest => {
            let func = state.storage.get_latest_version(&function_name)?
                .ok_or(ApiError::FunctionNotFound)?;
            func.version
        }
    };
    
    // Get function by name and version
    let function = state.storage.get_function_version(&function_name, version)?
        .ok_or(ApiError::FunctionNotFound)?;
    
    // Execute with version-aware caching
    let result = match function.runtime.as_str() {
        "python" => state.python_executor.execute(function.id, version, input).await?,
        "nodejs" => state.nodejs_executor.execute(function.id, version, input).await?,
        _ => return Err(ApiError::UnsupportedRuntime),
    };
    
    Ok(Json(result))
}
```

---

## Migration Strategy

### Phase 1: Add Version Support (Backward Compatible)
1. Add `version` and `is_latest` columns to functions table (default = 1, TRUE)
2. Add UNIQUE constraint on (name, version)
3. Update `create_function` to set version = 1
4. Update `get_function` to filter by `is_latest = TRUE`
5. **All existing functions become v1 automatically**

### Phase 2: Version Publishing
1. Add `publish_version` method
2. Add `PUT /functions/{name}?publish=true` endpoint
3. Update runtime to use (id, version) cache key
4. Test version isolation

### Phase 3: Alias Support
1. Create alias tables
2. Add alias CRUD endpoints
3. Add alias resolution in invoke handler
4. Add traffic splitting logic

### Phase 4: Advanced Features
1. Gradual rollout (traffic weights)
2. Auto-rollback on errors
3. Version retention policies
4. Version comparison/diff

---

## API Examples

### Creating and Publishing Versions

```bash
# 1. Create function (v1)
curl -X POST http://localhost:8080/functions \
  -H "Content-Type: application/json" \
  -d '{
    "name": "my-api",
    "runtime": "python",
    "handler": "handler",
    "code": "def handler(e,c): return {\"version\": 1}"
  }'
# Response: {"name": "my-api", "version": 1}

# 2. Invoke v1
curl -X POST http://localhost:8080/functions/my-api/invoke \
  -d '{}'
# Response: {"body": {"version": 1}}

# 3. Publish v2 (update code)
curl -X PUT http://localhost:8080/functions/my-api?publish=true \
  -H "Content-Type: application/json" \
  -d '{
    "code": "def handler(e,c): return {\"version\": 2, \"improved\": true}"
  }'
# Response: {"name": "my-api", "version": 2}

# 4. List versions
curl http://localhost:8080/functions/my-api/versions
# Response: {
#   "versions": [
#     {"version": 2, "is_latest": true, "published_at": 1234567890},
#     {"version": 1, "is_latest": false, "published_at": 1234567800}
#   ]
# }

# 5. Invoke specific versions
curl -X POST http://localhost:8080/functions/my-api:1/invoke -d '{}'
# Response: {"body": {"version": 1}}  ← Old version still works!

curl -X POST http://localhost:8080/functions/my-api:2/invoke -d '{}'
# Response: {"body": {"version": 2, "improved": true}}

curl -X POST http://localhost:8080/functions/my-api:$LATEST/invoke -d '{}'
# Response: {"body": {"version": 2, "improved": true}}
```

### Using Aliases

```bash
# 1. Create production alias (points to v1)
curl -X POST http://localhost:8080/functions/my-api/aliases \
  -H "Content-Type: application/json" \
  -d '{
    "alias": "production",
    "version": 1
  }'

# 2. Production traffic uses v1
curl -X POST http://localhost:8080/functions/my-api:production/invoke -d '{}'
# Response: {"body": {"version": 1}}

# 3. Test v2 on $LATEST
curl -X POST http://localhost:8080/functions/my-api:$LATEST/invoke -d '{}'
# Response: {"body": {"version": 2, "improved": true}}

# 4. v2 looks good! Promote to production
curl -X PUT http://localhost:8080/functions/my-api/aliases/production \
  -H "Content-Type: application/json" \
  -d '{
    "version": 2
  }'

# 5. Production traffic now uses v2
curl -X POST http://localhost:8080/functions/my-api:production/invoke -d '{}'
# Response: {"body": {"version": 2, "improved": true}}

# 6. Found a bug! Rollback to v1
curl -X PUT http://localhost:8080/functions/my-api/aliases/production \
  -H "Content-Type: application/json" \
  -d '{
    "version": 1
  }'
```

### Gradual Rollout

```bash
# Start with 10% traffic to v2, 90% to v1
curl -X PUT http://localhost:8080/functions/my-api/aliases/production \
  -H "Content-Type: application/json" \
  -d '{
    "routing": [
      {"version": 1, "weight": 0.9},
      {"version": 2, "weight": 0.1}
    ]
  }'

# Monitor metrics for v2...
# If good, increase to 50/50
curl -X PUT http://localhost:8080/functions/my-api/aliases/production \
  -H "Content-Type: application/json" \
  -d '{
    "routing": [
      {"version": 1, "weight": 0.5},
      {"version": 2, "weight": 0.5}
    ]
  }'

# If still good, go 100% v2
curl -X PUT http://localhost:8080/functions/my-api/aliases/production \
  -H "Content-Type: application/json" \
  -d '{
    "version": 2
  }'
```

---

## Benefits

### For the Current Bug
✅ **Solves process pool issue**: Each version has its own process pool  
✅ **Clean separation**: v1 and v2 run in different processes  
✅ **No code conflicts**: Updating to v2 doesn't affect v1  

### For Production Use
✅ **Zero-downtime deployments**: Switch alias atomically  
✅ **Easy rollback**: Just update alias, no code rebuild  
✅ **A/B testing**: Route different users to different versions  
✅ **Gradual rollout**: Test with 1% traffic before full deployment  
✅ **Multiple environments**: dev/staging/prod use different versions  
✅ **Audit trail**: All versions preserved with timestamps  

### For Compliance
✅ **Reproducibility**: Can always re-run exact version  
✅ **Disaster recovery**: Old versions available for rollback  
✅ **Change tracking**: Clear history of all deployments  

---

## Best Practices Alignment

### AWS Lambda
✅ Uses versions + aliases (exactly our model)  
✅ `$LATEST` for development  
✅ Numbered versions (1, 2, 3...) for production  
✅ Aliases for environment management  

### Google Cloud Functions
✅ Uses revisions (similar to versions)  
✅ Traffic splitting between revisions  
✅ Blue-green deployments  

### Azure Functions
✅ Uses deployment slots (similar to aliases)  
✅ Swap slots for zero-downtime deployment  

### Kubernetes
✅ Uses rolling updates with version tracking  
✅ Easy rollback to previous versions  
✅ Gradual traffic shifting (canary deployments)  

---

## Implementation Estimate

### Effort
- **Phase 1 (Versioning)**: 2-3 days
  - Database migration
  - Storage layer changes
  - Runtime cache key update
  - API endpoint updates
  - Tests

- **Phase 2 (Aliases)**: 1-2 days
  - Alias tables
  - CRUD operations
  - Resolution logic
  - Tests

- **Phase 3 (Traffic Splitting)**: 1-2 days
  - Weighted routing
  - Random distribution
  - Monitoring
  - Tests

**Total**: 4-7 days for full implementation

### Testing Requirements
- ✅ Version isolation (v1 doesn't affect v2)
- ✅ Process pool separation (different cache keys)
- ✅ Alias routing correctness
- ✅ Traffic weight distribution
- ✅ Rollback scenarios
- ✅ Migration from single version to multi-version

---

## Recommendation

### ✅ YES - Implement Function Versioning

**Reasons**:
1. **Industry standard** - All major serverless platforms use this
2. **Solves current bug** - Process pool code update issue goes away
3. **Production ready** - Essential for real-world deployments
4. **Low risk** - Can be added backward-compatibly
5. **High value** - Enables advanced deployment strategies

### Implementation Priority
1. ✅ **Phase 1** (Versioning) - **HIGH** - Solves the code update bug
2. ✅ **Phase 2** (Aliases) - **HIGH** - Essential for production use
3. ⭐ **Phase 3** (Traffic Splitting) - **MEDIUM** - Nice to have

### Backward Compatibility
✅ All existing functions become v1 automatically  
✅ Invokes without version specifier use $LATEST  
✅ No breaking changes to existing API  

---

## Next Steps

1. **Review this design** with stakeholders
2. **Create database migration** script
3. **Implement Phase 1** (versioning core)
4. **Add tests** for version isolation
5. **Update documentation**
6. **Implement Phase 2** (aliases)
7. **Add monitoring** for version usage
8. **Consider Phase 3** based on user needs

---

**Status**: 📋 Ready for Implementation  
**Approval Needed**: Yes  
**Breaking Changes**: None (backward compatible)  
**Timeline**: 4-7 days (phased approach)
