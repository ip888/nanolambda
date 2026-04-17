# Function Versioning API

**Status**: ✅ Production Ready  
**Compatible with**: AWS Lambda versioning model

---

## Overview

NanoLambda supports AWS Lambda-compatible function versioning. Each time you publish a new version, the old version remains available for rollback or parallel execution.

**Key Features**:
- ✅ Automatic version management (v1, v2, v3...)
- ✅ Version isolation in process pool (no code conflicts)
- ✅ Immutable versions (old versions never change)
- ✅ Latest version tracking
- ✅ Easy rollback capabilities

---

## API Endpoints

### 1. Create Function (v1)

Creates a new function starting at version 1.

```bash
POST /functions
```

**Request**:
```json
{
  "name": "my-function",
  "runtime": "python3.11",
  "handler": "handler",
  "code": "def handler(event, context): return {'version': 1}",
  "memory_mb": 128,
  "timeout_ms": 3000
}
```

**Response**:
```json
{
  "name": "my-function",
  "runtime": "python3.11",
  "status": "active",
  "created_at": 1732694400,
  "updated_at": 1732694400
}
```

---

### 2. Publish New Version

Publishes a new version of an existing function. Automatically increments version number and marks as latest.

```bash
POST /functions/{name}/versions
```

**Request**:
```json
{
  "runtime": "python3.11",
  "handler": "handler",
  "code": "def handler(event, context): return {'version': 2, 'updated': True}",
  "memory_mb": 128,
  "timeout_ms": 3000
}
```

**Response**:
```json
{
  "id": 42,
  "name": "my-function",
  "version": 2,
  "is_latest": true,
  "runtime": "python3.11",
  "handler": "handler",
  "code_hash": "a069ccd1...",
  "memory_mb": 128,
  "timeout_ms": 3000,
  "created_at": 1732694400,
  "updated_at": 1732698000
}
```

---

### 3. List All Versions

Returns all versions of a function, ordered by version DESC (newest first).

```bash
GET /functions/{name}/versions
```

**Response**:
```json
{
  "versions": [
    {
      "id": 42,
      "name": "my-function",
      "version": 2,
      "is_latest": true,
      "runtime": "python3.11",
      "handler": "handler",
      "code_hash": "a069ccd1...",
      "memory_mb": 128,
      "timeout_ms": 3000,
      "created_at": 1732694400,
      "updated_at": 1732698000
    },
    {
      "id": 41,
      "name": "my-function",
      "version": 1,
      "is_latest": false,
      "runtime": "python3.11",
      "handler": "handler",
      "code_hash": "014782c9...",
      "memory_mb": 128,
      "timeout_ms": 3000,
      "created_at": 1732694400,
      "updated_at": 1732694400
    }
  ],
  "count": 2
}
```

---

### 4. Get Specific Version

Retrieves a specific version of a function.

```bash
GET /functions/{name}/versions/{version}
```

**Example**: `GET /functions/my-function/versions/1`

**Response**:
```json
{
  "id": 41,
  "name": "my-function",
  "version": 1,
  "is_latest": false,
  "runtime": "python3.11",
  "handler": "handler",
  "code_hash": "014782c9...",
  "memory_mb": 128,
  "timeout_ms": 3000,
  "created_at": 1732694400,
  "updated_at": 1732694400
}
```

---

### 5. Invoke Latest Version

When you invoke without specifying a version, the latest version is used.

```bash
POST /functions/{name}/invoke
```

**Request**:
```json
{
  "payload": {"key": "value"}
}
```

**Response**:
```json
{
  "request_id": "uuid...",
  "status_code": 200,
  "body": {"version": 2, "updated": true},
  "metrics": {
    "execution_time_ms": 23,
    "memory_used_mb": 11.2,
    "cold_start": false
  }
}
```

---

## Complete Example Workflow

```bash
# 1. Create function (v1)
curl -X POST http://localhost:8080/functions \
  -H "Content-Type: application/json" \
  -d '{
    "name": "calc",
    "runtime": "python3.11",
    "handler": "handler",
    "code": "def handler(event, context): return event[\"x\"] * 2",
    "memory_mb": 128,
    "timeout_ms": 3000
  }'

# 2. Invoke v1 (returns x * 2)
curl -X POST http://localhost:8080/functions/calc/invoke \
  -H "Content-Type: application/json" \
  -d '{"payload": {"x": 10}}'
# Result: 20

# 3. Publish v2 with different logic
curl -X POST http://localhost:8080/functions/calc/versions \
  -H "Content-Type: application/json" \
  -d '{
    "runtime": "python3.11",
    "handler": "handler",
    "code": "def handler(event, context): return event[\"x\"] * 3",
    "memory_mb": 128,
    "timeout_ms": 3000
  }'

# 4. Invoke latest (v2, returns x * 3)
curl -X POST http://localhost:8080/functions/calc/invoke \
  -H "Content-Type: application/json" \
  -d '{"payload": {"x": 10}}'
# Result: 30

# 5. List all versions
curl http://localhost:8080/functions/calc/versions
# Shows v2 (latest) and v1

# 6. Get v1 specifically
curl http://localhost:8080/functions/calc/versions/1
# Returns v1 metadata (x * 2 code)
```

---

## Key Behaviors

### Version Isolation
- ✅ Each version has its own process pool entry: `cache_key = "function_id:version"`
- ✅ v1 and v2 execute different code simultaneously without conflicts
- ✅ Warm starts work independently per version

### Latest Version
- ✅ `is_latest` flag marks the current production version
- ✅ Only one version per function can be `is_latest = true`
- ✅ Publishing new version automatically updates the flag

### Immutability
- ✅ Once published, version code never changes
- ✅ To update code, publish a new version
- ✅ Old versions remain available for rollback

### Database Schema
```sql
CREATE TABLE functions (
  id INTEGER PRIMARY KEY,
  name TEXT NOT NULL,
  version INTEGER DEFAULT 1,
  is_latest BOOLEAN DEFAULT TRUE,
  code TEXT NOT NULL,
  code_hash TEXT NOT NULL,
  -- ... other fields
  UNIQUE(name, version)  -- allows multiple versions per function
);
```

---

## Error Responses

### 404 Not Found
```json
{
  "error": "VersionNotFound",
  "message": "Function 'my-function' version 5 not found"
}
```

### 500 Internal Server Error
```json
{
  "error": "StorageError",
  "message": "Failed to publish version: ..."
}
```

---

## Comparison with AWS Lambda

| Feature | AWS Lambda | NanoLambda | Status |
|---------|-----------|------------|--------|
| Version numbering | ✅ 1, 2, 3... | ✅ 1, 2, 3... | ✅ Same |
| $LATEST concept | ✅ Yes | ✅ Yes (is_latest) | ✅ Same |
| Immutable versions | ✅ Yes | ✅ Yes | ✅ Same |
| Version aliases | ✅ Yes | ⏳ Coming soon | 🔄 Future |
| Traffic splitting | ✅ Yes | ⏳ Coming soon | 🔄 Future |

---

## Future Enhancements (Phase 2)

- [ ] Version aliases (e.g., "prod", "staging")
- [ ] Traffic splitting between versions (90% v1, 10% v2)
- [ ] Automatic rollback on errors
- [ ] Version-specific metrics

---

**Status**: Core versioning ✅ Complete | Advanced features 🔄 Roadmap
