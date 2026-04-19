# How Users Deploy and Run Code on NanoLambda

## Simple 3-Step Process

```
1. Create API Key → 2. Deploy Function → 3. Invoke Function
```

## Step-by-Step Guide

### Step 1: Create API Key (One-time)

**What it does:** Gives user authentication credentials

```bash
curl -X POST http://localhost:8080/api/keys \
  -H "Content-Type: application/json" \
  -d '{"name": "my-app-key"}'
```

**Response:**
```json
{
  "id": "550e8400-e29b-41d4-a716-446655440000",
  "key": "nl_1234567890abcdef",
  "name": "my-app-key",
  "created_at": "2025-12-17T00:00:00Z"
}
```

💡 **Save the key!** User needs it for all future requests.

---

### Step 2: Deploy Function Code

**What it does:** Uploads code and configuration to NanoLambda

#### Python Example
```bash
curl -X POST http://localhost:8080/api/functions \
  -H "Content-Type: application/json" \
  -H "X-API-Key: nl_1234567890abcdef" \
  -d '{
    "name": "hello-world",
    "runtime": "python",
    "handler": "main.handler",
    "code": "def handler(event, context):\n    name = event.get(\"name\", \"World\")\n    return {\"message\": f\"Hello {name}!\"}",
    "memory_mb": 128,
    "timeout_ms": 3000,
    "environment": {
      "API_URL": "https://example.com"
    }
  }'
```

#### Node.js Example
```bash
curl -X POST http://localhost:8080/api/functions \
  -H "Content-Type: application/json" \
  -H "X-API-Key: nl_1234567890abcdef" \
  -d '{
    "name": "data-processor",
    "runtime": "nodejs",
    "handler": "index.handler",
    "code": "exports.handler = async (event) => {\n  return { processed: true, data: event };\n};",
    "memory_mb": 256,
    "timeout_ms": 5000
  }'
```

#### Java Example
```bash
curl -X POST http://localhost:8080/api/functions \
  -H "Content-Type: application/json" \
  -H "X-API-Key: nl_1234567890abcdef" \
  -d '{
    "name": "batch-processor",
    "runtime": "java",
    "handler": "com.example.Handler",
    "code": "package com.example;\npublic class Handler {\n  public String handleRequest(Map<String,Object> event) {\n    return \"Processed\";\n  }\n}",
    "memory_mb": 512,
    "timeout_ms": 10000
  }'
```

**What happens behind the scenes:**
1. ✅ NanoLambda validates the code
2. ✅ Stores function in database
3. ✅ Prepares runtime environment
4. ✅ Creates warm process pool (optional)
5. ✅ Returns function ID

---

### Step 3: Invoke Function

**What it does:** Executes the user's code with provided input

```bash
curl -X POST http://localhost:8080/api/functions/hello-world/invoke \
  -H "Content-Type: application/json" \
  -H "X-API-Key: nl_1234567890abcdef" \
  -d '{"name": "Alice"}'
```

**Response:**
```json
{
  "result": {
    "message": "Hello Alice!"
  },
  "execution_time_ms": 23,
  "cold_start": false,
  "invocation_id": "inv_abc123"
}
```

---

## Behind the Scenes: Execution Flow

```
User Request
    ↓
[API Server] ← Validates API key
    ↓
[Storage] ← Fetches function config
    ↓
[Runtime Executor] ← Selects appropriate runtime
    ↓
┌─────────────────────────────────┐
│  Runtime Selection:              │
│  • Python → Python Process Pool  │
│  • Node.js → Node Process Pool   │
│  • Java → JVM Process Pool       │
└─────────────────────────────────┘
    ↓
[Process Pool] ← Warm process ready? YES → Fast execution (12ms)
               ← Warm process ready? NO → Cold start (35ms)
    ↓
[Execute Code] ← Sandbox, resource limits, timeout
    ↓
[Collect Metrics] ← Duration, memory, status
    ↓
[Store Result] ← Save to database
    ↓
[Return Response] → User gets result
```

---

## Code Packaging Options

### Option 1: Inline Code (Simple Functions)
```json
{
  "code": "def handler(event, context): return {'result': 42}"
}
```
**Best for:** Small functions (<10KB)

### Option 2: Base64 Encoded (Medium Functions)
```json
{
  "code": "ZGVmIGhhbmRsZXIoZXZlbnQsIGNvbnRleHQpOgogICAgcmV0dXJuIHsncmVzdWx0JzogNDJ9"
}
```
**Best for:** Functions with dependencies (<1MB)

### Option 3: ZIP Upload (Large Projects)
```bash
# Package function
zip -r function.zip main.py requirements.txt

# Upload
curl -X POST http://localhost:8080/api/functions \
  -H "X-API-Key: nl_1234567890abcdef" \
  -F "name=complex-app" \
  -F "runtime=python" \
  -F "handler=main.handler" \
  -F "code=@function.zip"
```
**Best for:** Large applications with dependencies

---

## Function Configuration

### Required Fields
```json
{
  "name": "my-function",        // Unique identifier
  "runtime": "python|nodejs|java", // Language runtime
  "handler": "file.function",   // Entry point
  "code": "...",                // Source code
  "memory_mb": 128,             // Memory limit
  "timeout_ms": 3000            // Max execution time
}
```

### Optional Fields
```json
{
  "environment": {              // Environment variables
    "API_KEY": "secret123",
    "DEBUG": "true"
  },
  "description": "Processes payments", // Documentation
  "version": "1.0.0"            // Version tracking
}
```

---

## Runtime Features

### Python Runtime
- **Supported versions:** 3.12, 3.13
- **Pre-installed packages:** requests, boto3, numpy, pandas
- **Custom packages:** Upload with requirements.txt

### Node.js Runtime
- **Supported versions:** 14.x, 16.x, 18.x, 20.x
- **Pre-installed packages:** axios, lodash, moment
- **Custom packages:** Upload with package.json

### Java Runtime
- **Supported versions:** Java 11, 17, 21
- **Frameworks:** Spring Boot, Micronaut
- **Build tools:** Maven, Gradle

---

## Development Workflow

### 1. Local Development
```bash
# Write code locally
vim my_function.py

# Test locally (optional: NanoLambda CLI)
nanolambda test my_function.py

# Deploy
nanolambda deploy my_function.py
```

### 2. CI/CD Integration
```yaml
# GitHub Actions example
name: Deploy to NanoLambda

on: push

jobs:
  deploy:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v2
      - name: Deploy function
        run: |
          curl -X POST $NANOLAMBDA_URL/api/functions \
            -H "X-API-Key: ${{ secrets.NANOLAMBDA_KEY }}" \
            -d @function.json
```

### 3. Version Management
```bash
# Deploy new version
curl -X PUT http://localhost:8080/api/functions/my-func/versions \
  -H "X-API-Key: nl_xxx" \
  -d '{"version": "2.0.0", "code": "..."}'

# Rollback
curl -X POST http://localhost:8080/api/functions/my-func/rollback \
  -H "X-API-Key: nl_xxx" \
  -d '{"version": "1.0.0"}'
```

---

## Monitoring & Debugging

### View Function Logs
```bash
curl http://localhost:8080/api/functions/my-func/logs \
  -H "X-API-Key: nl_xxx"
```

### Get Metrics
```bash
curl http://localhost:8080/api/functions/my-func/metrics \
  -H "X-API-Key: nl_xxx"
```

### Live Dashboard
Visit: `http://localhost:8080/dashboard`
- Real-time invocations
- Latency graphs
- Error rates
- Cold start statistics

---

## Error Handling

### Common Errors

**1. Authentication Failed**
```json
{
  "error": "Invalid API key"
}
```
**Fix:** Check API key is correct

**2. Function Not Found**
```json
{
  "error": "Function 'my-func' not found"
}
```
**Fix:** Deploy function first

**3. Timeout**
```json
{
  "error": "Function exceeded timeout (3000ms)"
}
```
**Fix:** Increase timeout or optimize code

**4. Memory Limit**
```json
{
  "error": "Function exceeded memory limit (128MB)"
}
```
**Fix:** Increase memory_mb

---

## Best Practices

### 1. Keep Functions Small
✅ Single responsibility
✅ < 50 lines of code
✅ Fast startup time

### 2. Set Appropriate Limits
```json
{
  "memory_mb": 128,     // Start small
  "timeout_ms": 3000    // 3 seconds for most
}
```

### 3. Use Environment Variables
```json
{
  "environment": {
    "DB_URL": "postgres://...",
    "API_KEY": "secret"
  }
}
```

### 4. Handle Errors Gracefully
```python
def handler(event, context):
    try:
        # Your code
        return {"success": True}
    except Exception as e:
        return {"error": str(e)}
```

### 5. Test Before Deploy
- Test locally first
- Use staging environment
- Monitor metrics after deploy

---

## SDK & CLI (Coming Soon)

### Python SDK
```python
from nanolambda import Client

client = Client(api_key="nl_xxx")

# Deploy
client.deploy_function(
    name="my-func",
    code="def handler(event, context): ...",
    runtime="python"
)

# Invoke
result = client.invoke("my-func", {"input": "data"})
print(result)
```

### CLI
```bash
# Initialize project
nanolambda init my-project

# Deploy
nanolambda deploy

# Invoke
nanolambda invoke my-func '{"input": "data"}'

# Logs
nanolambda logs my-func --tail
```

---

## Summary: User Journey

```
1. Sign up / Get API Key (1 minute)
   ↓
2. Write function code (5-30 minutes)
   ↓
3. Deploy via API (10 seconds)
   ↓
4. Invoke function (10-50ms)
   ↓
5. Monitor dashboard (real-time)
   ↓
6. Scale automatically (no config needed)
```

**Total time from signup to running function: < 5 minutes!**

Compare to AWS Lambda: 2-4 hours of setup! 🎯
