# 🚀 Getting Started with NanoLambda

**Quick start guide for new users** - Step-by-step testing of all supported languages

---

## Prerequisites

- NanoLambda server running on `http://localhost:8080`
- `curl` and `jq` installed
- Terminal/command line access

---

## Step 1: Verify Server is Running

**Command:**
```bash
curl -s http://localhost:8080/health | jq .
```

**What it does:** Checks if the API server is responding  
**Expected output:** `{"status": "healthy", "version": "0.1.0"}`

---

## Step 2: Create an API Key

**Command:**
```bash
curl -X POST http://localhost:8080/auth/keys \
  -H "Content-Type: application/json" \
  -d '{"name": "test-key", "permissions": ["functions:create", "functions:invoke"]}' | jq .
```

**Parameters explained:**
- `name`: Just a label for this key (any string you want)
- `permissions`: Array of what this key can do
  - `functions:create` = can create functions
  - `functions:invoke` = can execute functions

**Expected output:** You'll get a `key` field like `nl_abc123...` - **SAVE THIS!**

**Save the key for next commands:**
```bash
export API_KEY="nl_YOUR_KEY_HERE"
```
*(Replace `nl_YOUR_KEY_HERE` with the actual key from the output)*

---

## Step 3: Python - Simplest Function

**Command:**
```bash
curl -X POST http://localhost:8080/functions \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer $API_KEY" \
  -d '{
    "name": "py-hello",
    "runtime": "python3.12",
    "handler": "handler",
    "memory_mb": 128,
    "timeout_ms": 30000,
    "code": "def handler(event, context):\n    return {\"statusCode\": 200, \"body\": \"Hello World\"}"
  }' | jq .
```

**Parameters explained:**
- `name`: Unique name for your function (use this to call it later)
- `runtime`: `python3.12` = Python 3.12 interpreter
- `handler`: `handler` = name of the Python function to call
- `memory_mb`: 128 = RAM limit in megabytes
- `timeout_ms`: 30000 = 30 seconds max execution time
- `code`: Your Python code as a JSON string
  - `\n` = newline character
  - `\"` = escaped quotes inside JSON

**The Python code explained:**
```python
def handler(event, context):    # Function name must match "handler" parameter
    return {
        'statusCode': 200,       # HTTP-like status code
        'body': 'Hello World'    # Response body
    }
```

**Expected output:** Function details showing `status: "active"`

---

## Step 4: Invoke the Python Function (Test 1 - Cold Start)

**Command:**
```bash
curl -X POST http://localhost:8080/functions/py-hello/invoke \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer $API_KEY" \
  -d '{"payload": {}}' | jq .
```

**Parameters explained:**
- `/functions/py-hello/invoke` = calls the function named "py-hello"
- `payload`: The `event` object passed to your handler function
  - `{}` = empty object (we're not passing any data yet)

**Expected output:**
```json
{
  "request_id": "uuid-here",
  "status_code": 200,
  "body": {
    "statusCode": 200,
    "body": "Hello World"
  },
  "metrics": {
    "execution_time_ms": <number>,
    "memory_used_mb": <number>,
    "cold_start": true    ← First call, process needs to start
  }
}
```

---

## Step 5: Invoke Again (Test 2 - Warm Start)

**Command:** *(same as above)*
```bash
curl -X POST http://localhost:8080/functions/py-hello/invoke \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer $API_KEY" \
  -d '{"payload": {}}' | jq .
```

**Expected difference:**
- `"cold_start": false` ← Process already running, instant!
- `"execution_time_ms"` should be much lower (often 0)

---

## Step 6: Python - Function with Input

**Command:**
```bash
curl -X POST http://localhost:8080/functions \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer $API_KEY" \
  -d '{
    "name": "py-greet",
    "runtime": "python3.12",
    "handler": "handler",
    "memory_mb": 128,
    "timeout_ms": 30000,
    "code": "def handler(event, context):\n    name = event.get(\"name\", \"World\")\n    return {\"statusCode\": 200, \"body\": f\"Hello, {name}!\"}"
  }' | jq .
```

**The code explained:**
```python
def handler(event, context):
    name = event.get("name", "World")  # Get "name" from event, default to "World"
    return {
        "statusCode": 200,
        "body": f"Hello, {name}!"      # Use Python f-string to insert name
    }
```

---

## Step 7: Test with Different Inputs

**Test 1 - No input:**
```bash
curl -X POST http://localhost:8080/functions/py-greet/invoke \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer $API_KEY" \
  -d '{"payload": {}}' | jq .
```
**Expected:** `"body": "Hello, World!"`

**Test 2 - With name:**
```bash
curl -X POST http://localhost:8080/functions/py-greet/invoke \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer $API_KEY" \
  -d '{"payload": {"name": "Alice"}}' | jq .
```
**Expected:** `"body": "Hello, Alice!"`

**Test 3 - Different name:**
```bash
curl -X POST http://localhost:8080/functions/py-greet/invoke \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer $API_KEY" \
  -d '{"payload": {"name": "Bob"}}' | jq .
```
**Expected:** `"body": "Hello, Bob!"`

---

## Step 8: Python - Math Calculator

**Command:**
```bash
curl -X POST http://localhost:8080/functions \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer $API_KEY" \
  -d '{
    "name": "py-calc",
    "runtime": "python3.12",
    "handler": "handler",
    "memory_mb": 128,
    "timeout_ms": 30000,
    "code": "def handler(event, context):\n    a = event.get(\"a\", 0)\n    b = event.get(\"b\", 0)\n    op = event.get(\"op\", \"add\")\n    if op == \"add\":\n        result = a + b\n    elif op == \"multiply\":\n        result = a * b\n    else:\n        return {\"statusCode\": 400, \"body\": \"Unknown operation\"}\n    return {\"statusCode\": 200, \"body\": {\"result\": result}}"
  }' | jq .
```

**The code explained:**
```python
def handler(event, context):
    a = event.get("a", 0)           # First number
    b = event.get("b", 0)           # Second number
    op = event.get("op", "add")     # Operation: "add" or "multiply"
    
    if op == "add":
        result = a + b
    elif op == "multiply":
        result = a * b
    else:
        return {"statusCode": 400, "body": "Unknown operation"}
    
    return {"statusCode": 200, "body": {"result": result}}
```

---

## Step 9: Test Calculator

**Addition:**
```bash
curl -X POST http://localhost:8080/functions/py-calc/invoke \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer $API_KEY" \
  -d '{"payload": {"a": 5, "b": 3, "op": "add"}}' | jq .
```
**Expected:** `"result": 8`

**Multiplication:**
```bash
curl -X POST http://localhost:8080/functions/py-calc/invoke \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer $API_KEY" \
  -d '{"payload": {"a": 5, "b": 3, "op": "multiply"}}' | jq .
```
**Expected:** `"result": 15`

**Invalid operation:**
```bash
curl -X POST http://localhost:8080/functions/py-calc/invoke \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer $API_KEY" \
  -d '{"payload": {"a": 5, "b": 3, "op": "divide"}}' | jq .
```
**Expected:** `"body": "Unknown operation"` and `"status_code": 400`

---

## Step 10: Node.js - Simple Function

**Command:**
```bash
curl -X POST http://localhost:8080/functions \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer $API_KEY" \
  -d '{
    "name": "node-hello",
    "runtime": "nodejs22.x",
    "handler": "handler",
    "memory_mb": 128,
    "timeout_ms": 30000,
    "code": "exports.handler = async function(event, context) {\n    return {statusCode: 200, body: \"Hello from Node.js\"};\n};"
  }' | jq .
```

**IMPORTANT - Node.js uses CommonJS syntax:**
- `exports.handler = ...` NOT `export function handler`
- Must use `exports.handler` for the wrapper to find it

**The code explained:**
```javascript
exports.handler = async function(event, context) {
    return {
        statusCode: 200,
        body: "Hello from Node.js"
    };
};
```

---

## Step 11: Test Node.js Function

**Command:**
```bash
curl -X POST http://localhost:8080/functions/node-hello/invoke \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer $API_KEY" \
  -d '{"payload": {}}' | jq .
```

**Expected output:**
- `"status_code": 200`
- `"body": "Hello from Node.js"`
- First call: `"cold_start": true` (slower)
- Second call: `"cold_start": false` (faster)

---

## Step 12: Node.js - With Input

**Command:**
```bash
curl -X POST http://localhost:8080/functions \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer $API_KEY" \
  -d '{
    "name": "node-greet",
    "runtime": "nodejs22.x",
    "handler": "handler",
    "memory_mb": 128,
    "timeout_ms": 30000,
    "code": "exports.handler = async function(event, context) {\n    const name = event?.name || \"World\";\n    return {statusCode: 200, body: `Hello, ${name} from Node.js!`};\n};"
  }' | jq .
```

**The code explained:**
```javascript
exports.handler = async function(event, context) {
    const name = event?.name || "World";  // Get name or default to "World"
    return {
        statusCode: 200,
        body: `Hello, ${name} from Node.js!`  // Template string
    };
};
```

---

## Step 13: Test Node.js with Input

**No input:**
```bash
curl -X POST http://localhost:8080/functions/node-greet/invoke \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer $API_KEY" \
  -d '{"payload": {}}' | jq .
```

**With input:**
```bash
curl -X POST http://localhost:8080/functions/node-greet/invoke \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer $API_KEY" \
  -d '{"payload": {"name": "JavaScript Developer"}}' | jq .
```

---

## Step 14: Python - Using Context

**Command:**
```bash
curl -X POST http://localhost:8080/functions \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer $API_KEY" \
  -d '{
    "name": "py-context",
    "runtime": "python3.12",
    "handler": "handler",
    "memory_mb": 128,
    "timeout_ms": 30000,
    "code": "def handler(event, context):\n    info = {\n        \"function_name\": context.function_name,\n        \"memory_limit\": context.memory_limit_in_mb,\n        \"remaining_time\": context.get_remaining_time_in_millis()\n    }\n    return {\"statusCode\": 200, \"body\": info}"
  }' | jq .
```

**The code explained:**
```python
def handler(event, context):
    # context object provides execution environment info
    info = {
        "function_name": context.function_name,           # Name of this function
        "memory_limit": context.memory_limit_in_mb,      # RAM limit
        "remaining_time": context.get_remaining_time_in_millis()  # Time left
    }
    return {"statusCode": 200, "body": info}
```

**Test it:**
```bash
curl -X POST http://localhost:8080/functions/py-context/invoke \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer $API_KEY" \
  -d '{"payload": {}}' | jq .
```

**Expected output:** You'll see function name, memory limit, and remaining execution time

---

## Step 15: List All Your Functions

**Command:**
```bash
curl -X GET http://localhost:8080/functions \
  -H "Authorization: Bearer $API_KEY" | jq '.functions[] | {name, runtime, status}'
```

**What it shows:** All functions you created with their name, runtime, and status

---

## 📊 Quick Reference

### Function Creation Parameters

- `name`: Your unique function ID (alphanumeric + hyphens)
- `runtime`: 
  - `python3.12` for Python
  - `nodejs22.x` for Node.js
  - `java21` for Java (experimental)
- `handler`: Function name to call (usually just `"handler"`)
- `memory_mb`: RAM limit (128, 256, 512, etc.)
- `timeout_ms`: Max time in milliseconds (30000 = 30 seconds)
- `code`: Your code as a JSON string with `\n` for newlines

### Invocation

- `payload`: Object passed as `event` to your function
- Can be `{}` (empty) or `{"key": "value", ...}`

### Python Code Format

```python
def handler(event, context):
    # event = the payload you send (dict)
    # context = execution context (object with properties)
    return {"statusCode": 200, "body": "result"}
```

### Node.js Code Format

```javascript
exports.handler = async function(event, context) {
    // event = the payload you send (object)
    // context = execution context (object with properties)
    return {statusCode: 200, body: "result"};
};
```

---

## 🎯 Common Patterns

### Error Handling (Python)

```python
def handler(event, context):
    try:
        # Your logic here
        result = process_data(event)
        return {"statusCode": 200, "body": result}
    except Exception as e:
        return {"statusCode": 500, "body": f"Error: {str(e)}"}
```

### Error Handling (Node.js)

```javascript
exports.handler = async function(event, context) {
    try {
        // Your logic here
        const result = await processData(event);
        return {statusCode: 200, body: result};
    } catch (error) {
        return {statusCode: 500, body: `Error: ${error.message}`};
    }
};
```

### Using External Libraries (Python)

```python
import json
import statistics

def handler(event, context):
    data = event.get("numbers", [])
    avg = statistics.mean(data)
    return {"statusCode": 200, "body": {"average": avg}}
```

---

## 🐛 Troubleshooting

### "SyntaxError: invalid syntax"

**Problem:** Your Python code has syntax errors  
**Solution:** Check for:
- Proper indentation (4 spaces)
- Escaped quotes in JSON (`\"` instead of `"`)
- Newlines (`\n`) between lines

### "Handler function not found"

**Problem:** Node.js can't find your handler  
**Solution:** Use `exports.handler = ...` not `export function`

### "401 Unauthorized"

**Problem:** Missing or invalid API key  
**Solution:** 
1. Create a new API key (Step 2)
2. Export it: `export API_KEY="nl_your_key"`
3. Include header: `-H "Authorization: Bearer $API_KEY"`

### "Function already exists"

**Problem:** Function name is taken  
**Solution:** Use a different `name` or delete the old function first

---

## 🚀 Next Steps

1. **Try more complex functions** with libraries and data processing
2. **Test performance** by calling functions multiple times (warm vs cold starts)
3. **Build a real API** by creating multiple functions that work together
4. **Monitor metrics** to see execution time and memory usage

---

## 📚 See Also

- [LANGUAGE_SUPPORT_AUDIT.md](LANGUAGE_SUPPORT_AUDIT.md) - Full language support details
- [HANDLER_PARAMETERS.md](HANDLER_PARAMETERS.md) - Deep dive on `event` and `context`
- [COMPETITIVE_DIFFERENTIATORS.md](COMPETITIVE_DIFFERENTIATORS.md) - What makes NanoLambda unique

---

**Need help?** Check server logs or ask in the community!
