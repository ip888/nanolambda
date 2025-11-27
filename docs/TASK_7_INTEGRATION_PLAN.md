# Task 7: StorageManager Integration Plan

**Objective**: Wire the StorageManager into the API server to enable persistent function management

**Date**: October 18, 2025  
**Status**: In Progress  

---

## 📋 Current State

### What We Have

**Storage Layer** (`crates/storage/`):
- ✅ `StorageManager` with SQLite backend
- ✅ Function CRUD operations
- ✅ Invocation tracking
- ✅ Connection pooling with r2d2
- ✅ 7 passing tests
- ✅ ~900 lines of code

**Runtime Layer** (`crates/runtime/`):
- ✅ `Runtime` trait (language-agnostic)
- ✅ `PythonExecutor` implementation
- ✅ `NodeJSExecutor` implementation
- ✅ Process pooling
- ✅ Real metrics
- ✅ 36 passing tests

**API Server** (`crates/api-server/`):
- ✅ Basic Axum routes
- ✅ Hardcoded function execution
- ❌ **NOT** connected to storage
- ❌ **NOT** using Runtime trait
- ❌ Only Python executor

### The Gap

Current flow (BROKEN):
```
User Request → API Handler → Hardcoded Code → PythonExecutor → Response
```

Desired flow (END-TO-END):
```
User Request → API Handler → StorageManager → Runtime (multi-language) → Response
                            ↓
                       Database (SQLite)
```

---

## 🎯 Implementation Plan

### Phase 1: Update API Server State (30 min)
- [ ] Add `StorageManager` to `ApiServer` struct
- [ ] Support multiple runtime executors (Python, Node.js)
- [ ] Add database path configuration
- [ ] Update initialization

### Phase 2: Function Registration Endpoints (45 min)
- [ ] `POST /functions` - Create function
- [ ] `GET /functions` - List all functions
- [ ] `GET /functions/{name}` - Get function details
- [ ] `PUT /functions/{name}` - Update function
- [ ] `DELETE /functions/{name}` - Delete function

### Phase 3: Function Execution Integration (60 min)
- [ ] Update `invoke_function` to load from database
- [ ] Detect runtime from function config
- [ ] Route to appropriate executor (Python/Node.js)
- [ ] Record invocation in database
- [ ] Return execution metrics

### Phase 4: Error Handling (20 min)
- [ ] Handle function not found (404)
- [ ] Handle invalid runtime (400)
- [ ] Handle execution errors (500)
- [ ] Add proper HTTP status codes

### Phase 5: Testing (40 min)
- [ ] Integration tests for function CRUD
- [ ] Test Python function execution
- [ ] Test Node.js function execution
- [ ] Test error scenarios
- [ ] Test concurrent execution

### Phase 6: Documentation (25 min)
- [ ] API endpoint documentation
- [ ] Usage examples
- [ ] Migration guide
- [ ] Update README

**Total Estimated Time**: ~3.5 hours

---

## 🔧 Technical Details

### API State Structure

```rust
pub struct ApiServer {
    storage: Arc<StorageManager>,
    python_executor: Arc<Mutex<PythonExecutor>>,
    nodejs_executor: Arc<Mutex<NodeJSExecutor>>,
}
```

### New Endpoints

| Method | Path | Purpose |
|--------|------|---------|
| POST | `/functions` | Create new function |
| GET | `/functions` | List all functions |
| GET | `/functions/{name}` | Get function details |
| PUT | `/functions/{name}` | Update function |
| DELETE | `/functions/{name}` | Delete function |
| POST | `/functions/{name}/invoke` | Execute function |

### Request/Response Models

**Create Function Request**:
```json
{
  "name": "hello-world",
  "runtime": "python3.12",
  "handler": "lambda_function.handler",
  "code": "def handler(event, context): return {'message': 'Hello'}",
  "memory_mb": 128,
  "timeout_ms": 30000,
  "environment": {
    "KEY": "value"
  }
}
```

**Invoke Function Request**:
```json
{
  "payload": {
    "name": "World"
  }
}
```

**Invoke Function Response**:
```json
{
  "status_code": 200,
  "body": "{\"message\": \"Hello World\"}",
  "request_id": "abc-123",
  "metrics": {
    "execution_time_ms": 45,
    "memory_used_mb": 42,
    "cold_start": false
  }
}
```

---

## 🚀 Implementation Order

1. **Update Dependencies** (Cargo.toml)
   - Add `nanolambda-storage` to api-server
   - Add `nanolambda-runtime` with all features

2. **Refactor ApiServer Struct**
   - Add storage manager
   - Add both executors
   - Update initialization

3. **Implement Function Management**
   - Create function handler
   - List functions handler
   - Get function handler  
   - Update function handler
   - Delete function handler

4. **Refactor Invoke Handler**
   - Load function from database
   - Detect runtime
   - Route to correct executor
   - Record invocation
   - Return metrics

5. **Add Error Handling**
   - Custom error types
   - HTTP status mapping
   - Error responses

6. **Write Tests**
   - Unit tests for handlers
   - Integration tests
   - End-to-end tests

7. **Document Everything**
   - API documentation
   - Usage examples
   - Migration guide

---

## 📊 Success Criteria

- [ ] Functions can be registered via API
- [ ] Functions are persisted in SQLite
- [ ] Functions can be listed/retrieved
- [ ] Python functions execute successfully
- [ ] Node.js functions execute successfully
- [ ] Invocations are tracked in database
- [ ] Metrics are returned in response
- [ ] Error handling is comprehensive
- [ ] All tests pass
- [ ] Documentation is complete

---

## 🧪 Testing Strategy

### Unit Tests
- Storage manager integration
- Handler logic
- Error handling

### Integration Tests
```rust
#[tokio::test]
async fn test_create_and_invoke_python_function() {
    // Create function via API
    // Invoke function via API
    // Verify result
    // Check database records
}

#[tokio::test]
async fn test_create_and_invoke_nodejs_function() {
    // Similar for Node.js
}
```

### End-to-End Tests
- Complete user workflows
- Multiple functions
- Concurrent invocations
- Function updates

---

## 📝 Notes

- Keep backward compatibility where possible
- Use Arc for shared state
- Proper async/await usage
- Comprehensive error messages
- Return proper HTTP status codes
- Log all operations

---

**Ready to implement!** 🚀
