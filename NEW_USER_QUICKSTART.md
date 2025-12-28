# 🚀 New User Quickstart Guide
## Test NanoLambda with Real Functions

**Welcome!** This guide will walk you through testing NanoLambda from scratch.

---

## 📋 Prerequisites

- Rust installed (for building)
- Python 3.12+
- Node.js 22.x+
- Java 21+ (optional, for Java functions)

---

## 🎯 Quick Test Journey

We'll test functions in order of complexity:

### **Python**
1. Simple "Hello World" 
2. REST API with JSON
3. Data processing with computation

### **Node.js**
1. Simple async handler
2. Express-like API
3. Stream processing

### **Java**
1. Basic processor (if experimental support works)

---

## 🛠️ Step-by-Step Guide

### **Step 1: Build the Project**

```bash
# From project root
cargo build --release

# This compiles:
# - API server (crates/api-server)
# - Runtime executors (crates/runtime)
# - Storage layer (crates/storage)
```

**Expected output:**
```
   Compiling nanolambda-runtime v0.1.0
   Compiling nanolambda-storage v0.1.0
   Compiling nanolambda-api-server v0.1.0
    Finished release [optimized] target(s) in 2m 15s
```

---

### **Step 2: Start the API Server**

```bash
# Start server on default port 3000
cargo run --bin nanolambda-api-server --release
```

**Expected output:**
```
Starting NanoLambda API Server...
✓ Python 3.12 executor initialized
✓ Node.js 22.x executor initialized
✓ Java executor initialized (experimental)
✓ Storage layer ready
✓ API server listening on http://127.0.0.1:3000

Ready to accept function invocations! 🚀
```

**Keep this terminal open** - the server needs to run while you test.

---

### **Step 3: Open New Terminal for Testing**

Open a second terminal to send requests.

---

## 🐍 Test 1: Python "Hello World"

### **Create Simple Function**

```bash
mkdir -p /tmp/test-hello-python
cat > /tmp/test-hello-python/handler.py << 'EOF'
def handler(event, context):
    """Simple hello world function"""
    name = event.get('name', 'World')
    return {
        'statusCode': 200,
        'body': f'Hello, {name}! 🐍'
    }
EOF
```

### **Upload Function**

```bash
curl -X POST http://localhost:3000/functions \
  -H "Content-Type: application/json" \
  -d '{
    "name": "hello-python",
    "runtime": "python3.12",
    "handler": "handler.handler",
    "code_path": "/tmp/test-hello-python"
  }'
```

**Expected response:**
```json
{
  "function_id": "func_abc123...",
  "name": "hello-python",
  "runtime": "python3.12",
  "status": "created"
}
```

### **Invoke Function**

```bash
# Test 1: Default name
curl -X POST http://localhost:3000/functions/hello-python/invoke \
  -H "Content-Type: application/json" \
  -d '{}'

# Test 2: Custom name
curl -X POST http://localhost:3000/functions/hello-python/invoke \
  -H "Content-Type: application/json" \
  -d '{"name": "NanoLambda User"}'
```

**Expected responses:**
```json
{
  "statusCode": 200,
  "body": "Hello, World! 🐍",
  "metrics": {
    "duration_ms": 2.5,
    "memory_mb": 15.2,
    "cold_start": true
  }
}

{
  "statusCode": 200,
  "body": "Hello, NanoLambda User! 🐍",
  "metrics": {
    "duration_ms": 0.8,
    "memory_mb": 15.2,
    "cold_start": false
  }
}
```

**✅ Success!** Notice the second call is faster (warm start).

---

## 🐍 Test 2: Python REST API with JSON

### **Create REST API Function**

```bash
mkdir -p /tmp/test-rest-api
cat > /tmp/test-rest-api/handler.py << 'EOF'
import json
import time

def handler(event, context):
    """REST API endpoint with multiple operations"""
    
    # Parse request
    method = event.get('method', 'GET')
    path = event.get('path', '/')
    body = event.get('body', {})
    
    # Route handling
    if path == '/health':
        return {
            'statusCode': 200,
            'body': json.dumps({'status': 'healthy', 'timestamp': time.time()})
        }
    
    elif path == '/users' and method == 'POST':
        user_data = body
        return {
            'statusCode': 201,
            'body': json.dumps({
                'message': 'User created',
                'user': user_data,
                'id': 'user_123'
            })
        }
    
    elif path == '/calculate' and method == 'POST':
        a = body.get('a', 0)
        b = body.get('b', 0)
        operation = body.get('operation', 'add')
        
        operations = {
            'add': lambda x, y: x + y,
            'subtract': lambda x, y: x - y,
            'multiply': lambda x, y: x * y,
            'divide': lambda x, y: x / y if y != 0 else 'Error: Division by zero'
        }
        
        result = operations.get(operation, lambda x, y: 'Unknown operation')(a, b)
        
        return {
            'statusCode': 200,
            'body': json.dumps({
                'operation': operation,
                'a': a,
                'b': b,
                'result': result
            })
        }
    
    else:
        return {
            'statusCode': 404,
            'body': json.dumps({'error': 'Not found'})
        }
EOF
```

### **Upload and Test**

```bash
# Upload function
curl -X POST http://localhost:3000/functions \
  -H "Content-Type: application/json" \
  -d '{
    "name": "rest-api-python",
    "runtime": "python3.12",
    "handler": "handler.handler",
    "code_path": "/tmp/test-rest-api"
  }'

# Test 1: Health check
curl -X POST http://localhost:3000/functions/rest-api-python/invoke \
  -H "Content-Type: application/json" \
  -d '{"method": "GET", "path": "/health"}'

# Test 2: Create user
curl -X POST http://localhost:3000/functions/rest-api-python/invoke \
  -H "Content-Type: application/json" \
  -d '{
    "method": "POST",
    "path": "/users",
    "body": {"name": "Alice", "email": "alice@example.com"}
  }'

# Test 3: Calculate
curl -X POST http://localhost:3000/functions/rest-api-python/invoke \
  -H "Content-Type: application/json" \
  -d '{
    "method": "POST",
    "path": "/calculate",
    "body": {"a": 42, "b": 8, "operation": "multiply"}
  }'
```

---

## 🐍 Test 3: Python Data Processing (Complex)

### **Create Data Processing Function**

```bash
mkdir -p /tmp/test-data-processing
cat > /tmp/test-data-processing/handler.py << 'EOF'
import json
import statistics
import time

def handler(event, context):
    """Data processing with analytics"""
    
    # Get data from event
    data = event.get('data', [])
    operation = event.get('operation', 'analyze')
    
    if not data:
        return {
            'statusCode': 400,
            'body': json.dumps({'error': 'No data provided'})
        }
    
    start_time = time.time()
    
    if operation == 'analyze':
        # Statistical analysis
        result = {
            'count': len(data),
            'sum': sum(data),
            'mean': statistics.mean(data),
            'median': statistics.median(data),
            'min': min(data),
            'max': max(data),
            'range': max(data) - min(data)
        }
        
        if len(data) > 1:
            result['stdev'] = statistics.stdev(data)
            result['variance'] = statistics.variance(data)
    
    elif operation == 'filter':
        # Filter data based on threshold
        threshold = event.get('threshold', 0)
        filtered = [x for x in data if x > threshold]
        result = {
            'original_count': len(data),
            'filtered_count': len(filtered),
            'filtered_data': filtered,
            'threshold': threshold
        }
    
    elif operation == 'transform':
        # Transform data (square all values)
        transformed = [x ** 2 for x in data]
        result = {
            'original': data,
            'transformed': transformed,
            'transformation': 'square'
        }
    
    else:
        return {
            'statusCode': 400,
            'body': json.dumps({'error': f'Unknown operation: {operation}'})
        }
    
    processing_time = (time.time() - start_time) * 1000  # ms
    
    return {
        'statusCode': 200,
        'body': json.dumps({
            'operation': operation,
            'result': result,
            'processing_time_ms': round(processing_time, 2)
        })
    }
EOF
```

### **Upload and Test**

```bash
# Upload function
curl -X POST http://localhost:3000/functions \
  -H "Content-Type: application/json" \
  -d '{
    "name": "data-processor-python",
    "runtime": "python3.12",
    "handler": "handler.handler",
    "code_path": "/tmp/test-data-processing"
  }'

# Test 1: Analyze data
curl -X POST http://localhost:3000/functions/data-processor-python/invoke \
  -H "Content-Type: application/json" \
  -d '{
    "operation": "analyze",
    "data": [10, 20, 30, 40, 50, 60, 70, 80, 90, 100]
  }'

# Test 2: Filter data
curl -X POST http://localhost:3000/functions/data-processor-python/invoke \
  -H "Content-Type: application/json" \
  -d '{
    "operation": "filter",
    "data": [5, 15, 25, 35, 45, 55, 65, 75, 85, 95],
    "threshold": 50
  }'

# Test 3: Transform data
curl -X POST http://localhost:3000/functions/data-processor-python/invoke \
  -H "Content-Type: application/json" \
  -d '{
    "operation": "transform",
    "data": [1, 2, 3, 4, 5]
  }'
```

---

## 🟢 Test 4: Node.js "Hello World"

### **Create Simple Node.js Function**

```bash
mkdir -p /tmp/test-hello-nodejs
cat > /tmp/test-hello-nodejs/handler.js << 'EOF'
export async function handler(event, context) {
    const name = event.name || 'World';
    
    return {
        statusCode: 200,
        body: `Hello, ${name}! 🟢 from Node.js ${process.version}`,
        timestamp: new Date().toISOString()
    };
}
EOF
```

### **Upload and Test**

```bash
# Upload function
curl -X POST http://localhost:3000/functions \
  -H "Content-Type: application/json" \
  -d '{
    "name": "hello-nodejs",
    "runtime": "nodejs22.x",
    "handler": "handler.handler",
    "code_path": "/tmp/test-hello-nodejs"
  }'

# Invoke function
curl -X POST http://localhost:3000/functions/hello-nodejs/invoke \
  -H "Content-Type: application/json" \
  -d '{"name": "JavaScript Developer"}'
```

---

## 🟢 Test 5: Node.js Express-like API

### **Create Express API Function**

```bash
mkdir -p /tmp/test-express-api
cat > /tmp/test-express-api/handler.js << 'EOF'
export async function handler(event, context) {
    const { method = 'GET', path = '/', body = {} } = event;
    
    // Simulate routing
    const routes = {
        'GET:/': async () => ({
            statusCode: 200,
            body: JSON.stringify({ 
                message: 'Welcome to Express-like API',
                version: '1.0.0',
                endpoints: ['/users', '/posts', '/health']
            })
        }),
        
        'GET:/health': async () => ({
            statusCode: 200,
            body: JSON.stringify({
                status: 'healthy',
                uptime: process.uptime(),
                memory: process.memoryUsage(),
                node_version: process.version
            })
        }),
        
        'POST:/users': async () => ({
            statusCode: 201,
            body: JSON.stringify({
                message: 'User created',
                user: body,
                id: `user_${Date.now()}`
            })
        }),
        
        'GET:/users': async () => ({
            statusCode: 200,
            body: JSON.stringify({
                users: [
                    { id: 1, name: 'Alice' },
                    { id: 2, name: 'Bob' },
                    { id: 3, name: 'Charlie' }
                ]
            })
        }),
        
        'POST:/posts': async () => {
            // Simulate some async work
            await new Promise(resolve => setTimeout(resolve, 10));
            
            return {
                statusCode: 201,
                body: JSON.stringify({
                    message: 'Post created',
                    post: body,
                    id: `post_${Date.now()}`
                })
            };
        }
    };
    
    const routeKey = `${method}:${path}`;
    const handler = routes[routeKey];
    
    if (handler) {
        return await handler();
    } else {
        return {
            statusCode: 404,
            body: JSON.stringify({
                error: 'Not found',
                path,
                method
            })
        };
    }
}
EOF
```

### **Upload and Test**

```bash
# Upload function
curl -X POST http://localhost:3000/functions \
  -H "Content-Type: application/json" \
  -d '{
    "name": "express-api-nodejs",
    "runtime": "nodejs22.x",
    "handler": "handler.handler",
    "code_path": "/tmp/test-express-api"
  }'

# Test 1: Root endpoint
curl -X POST http://localhost:3000/functions/express-api-nodejs/invoke \
  -H "Content-Type: application/json" \
  -d '{"method": "GET", "path": "/"}'

# Test 2: Health check
curl -X POST http://localhost:3000/functions/express-api-nodejs/invoke \
  -H "Content-Type: application/json" \
  -d '{"method": "GET", "path": "/health"}'

# Test 3: Create user
curl -X POST http://localhost:3000/functions/express-api-nodejs/invoke \
  -H "Content-Type: application/json" \
  -d '{
    "method": "POST",
    "path": "/users",
    "body": {"name": "David", "email": "david@example.com"}
  }'

# Test 4: Get users
curl -X POST http://localhost:3000/functions/express-api-nodejs/invoke \
  -H "Content-Type: application/json" \
  -d '{"method": "GET", "path": "/users"}'
```

---

## 🟢 Test 6: Node.js Stream Processor (Complex)

### **Create Stream Processing Function**

```bash
mkdir -p /tmp/test-stream-processor
cat > /tmp/test-stream-processor/handler.js << 'EOF'
export async function handler(event, context) {
    const { operation = 'process', stream = [] } = event;
    
    const startTime = Date.now();
    
    // Async stream processing functions
    const processors = {
        process: async (stream) => {
            // Process stream items with async operations
            const results = await Promise.all(
                stream.map(async (item, index) => {
                    // Simulate async processing
                    await new Promise(resolve => setTimeout(resolve, 1));
                    
                    return {
                        index,
                        original: item,
                        processed: item * 2,
                        timestamp: Date.now()
                    };
                })
            );
            
            return {
                operation: 'process',
                itemCount: stream.length,
                results
            };
        },
        
        aggregate: async (stream) => {
            // Aggregate stream data
            const sum = stream.reduce((acc, val) => acc + val, 0);
            const avg = sum / stream.length;
            
            return {
                operation: 'aggregate',
                count: stream.length,
                sum,
                average: avg,
                min: Math.min(...stream),
                max: Math.max(...stream)
            };
        },
        
        batch: async (stream) => {
            // Process in batches
            const batchSize = event.batchSize || 3;
            const batches = [];
            
            for (let i = 0; i < stream.length; i += batchSize) {
                const batch = stream.slice(i, i + batchSize);
                
                // Process batch
                await new Promise(resolve => setTimeout(resolve, 5));
                
                batches.push({
                    batchNumber: Math.floor(i / batchSize) + 1,
                    items: batch,
                    sum: batch.reduce((a, b) => a + b, 0)
                });
            }
            
            return {
                operation: 'batch',
                batchSize,
                totalBatches: batches.length,
                batches
            };
        },
        
        filter: async (stream) => {
            // Filter stream based on condition
            const threshold = event.threshold || 50;
            const filtered = stream.filter(item => item > threshold);
            
            return {
                operation: 'filter',
                threshold,
                originalCount: stream.length,
                filteredCount: filtered.length,
                filtered
            };
        }
    };
    
    const processor = processors[operation];
    
    if (!processor) {
        return {
            statusCode: 400,
            body: JSON.stringify({
                error: `Unknown operation: ${operation}`,
                availableOperations: Object.keys(processors)
            })
        };
    }
    
    if (!stream || stream.length === 0) {
        return {
            statusCode: 400,
            body: JSON.stringify({
                error: 'No stream data provided'
            })
        };
    }
    
    try {
        const result = await processor(stream);
        const processingTime = Date.now() - startTime;
        
        return {
            statusCode: 200,
            body: JSON.stringify({
                ...result,
                processingTimeMs: processingTime,
                itemsPerMs: (stream.length / processingTime).toFixed(2)
            })
        };
    } catch (error) {
        return {
            statusCode: 500,
            body: JSON.stringify({
                error: error.message
            })
        };
    }
}
EOF
```

### **Upload and Test**

```bash
# Upload function
curl -X POST http://localhost:3000/functions \
  -H "Content-Type: application/json" \
  -d '{
    "name": "stream-processor-nodejs",
    "runtime": "nodejs22.x",
    "handler": "handler.handler",
    "code_path": "/tmp/test-stream-processor"
  }'

# Test 1: Process stream
curl -X POST http://localhost:3000/functions/stream-processor-nodejs/invoke \
  -H "Content-Type: application/json" \
  -d '{
    "operation": "process",
    "stream": [10, 20, 30, 40, 50]
  }'

# Test 2: Aggregate stream
curl -X POST http://localhost:3000/functions/stream-processor-nodejs/invoke \
  -H "Content-Type: application/json" \
  -d '{
    "operation": "aggregate",
    "stream": [15, 25, 35, 45, 55, 65, 75, 85, 95]
  }'

# Test 3: Batch processing
curl -X POST http://localhost:3000/functions/stream-processor-nodejs/invoke \
  -H "Content-Type: application/json" \
  -d '{
    "operation": "batch",
    "stream": [1, 2, 3, 4, 5, 6, 7, 8, 9, 10],
    "batchSize": 3
  }'

# Test 4: Filter stream
curl -X POST http://localhost:3000/functions/stream-processor-nodejs/invoke \
  -H "Content-Type: application/json" \
  -d '{
    "operation": "filter",
    "stream": [10, 25, 40, 55, 70, 85, 100],
    "threshold": 50
  }'
```

---

## ☕ Test 7: Java Simple Processor (Experimental)

**Note:** Java support is experimental. It may not work perfectly yet.

### **Create Java Function**

```bash
mkdir -p /tmp/test-java-processor
cat > /tmp/test-java-processor/Handler.java << 'EOF'
import java.util.Map;
import java.util.HashMap;

public class Handler {
    public Map<String, Object> handler(Map<String, Object> event, Object context) {
        Map<String, Object> response = new HashMap<>();
        
        String name = (String) event.getOrDefault("name", "World");
        
        response.put("statusCode", 200);
        response.put("body", "Hello, " + name + " from Java! ☕");
        response.put("runtime", "Java " + System.getProperty("java.version"));
        
        return response;
    }
}
EOF
```

### **Upload and Test**

```bash
# Upload function
curl -X POST http://localhost:3000/functions \
  -H "Content-Type: application/json" \
  -d '{
    "name": "hello-java",
    "runtime": "java21",
    "handler": "Handler.handler",
    "code_path": "/tmp/test-java-processor"
  }'

# Invoke function
curl -X POST http://localhost:3000/functions/hello-java/invoke \
  -H "Content-Type: application/json" \
  -d '{"name": "Java Developer"}'
```

**If this fails**, it's expected - Java is marked experimental and needs more work!

---

## 📊 Compare Performance Metrics

After running all tests, compare the metrics:

### **Cold Start Times:**
- Python: ~2-5ms
- Node.js: ~2-5ms
- Java: ~50-200ms (slower, needs optimization)

### **Warm Start Times:**
- Python: <1ms
- Node.js: <1ms
- Java: ~5-10ms

### **Memory Usage:**
- Python: ~15-30 MB
- Node.js: ~20-40 MB
- Java: ~60-150 MB (heavier JVM)

---

## 🎯 Success Criteria

You should see:
- ✅ All Python functions working perfectly
- ✅ All Node.js functions working perfectly
- ⚠️ Java function may or may not work (experimental)
- ✅ Sub-5ms cold starts for Python/Node.js
- ✅ Sub-1ms warm starts
- ✅ Accurate metrics in responses

---

## 🐛 Troubleshooting

### **Server won't start:**
```bash
# Check if port 3000 is in use
lsof -i :3000

# Kill existing process
kill -9 <PID>

# Try again
cargo run --bin nanolambda-api-server --release
```

### **Function upload fails:**
- Check that the code_path exists
- Verify the handler file exists (handler.py, handler.js, Handler.java)
- Check server logs for errors

### **Function invocation fails:**
- Check function was uploaded successfully
- Verify function name matches
- Look at server terminal for error messages

### **Java doesn't work:**
- This is expected! Java support is experimental
- See [LANGUAGE_SUPPORT_AUDIT.md](LANGUAGE_SUPPORT_AUDIT.md) for status

---

## 🎉 What You've Tested

Congratulations! You've tested:

1. ✅ **Python** - 3 functions (simple, REST API, data processing)
2. ✅ **Node.js** - 3 functions (simple, Express API, stream processing)
3. ⚠️ **Java** - 1 function (experimental)

**Total: 7 serverless functions across 3 language runtimes!**

---

## 📚 Next Steps

1. **Read the docs:**
   - [LANGUAGE_SUPPORT_AUDIT.md](LANGUAGE_SUPPORT_AUDIT.md) - Language support status
   - [COMPETITIVE_DIFFERENTIATORS.md](COMPETITIVE_DIFFERENTIATORS.md) - Unique features

2. **Try existing test suite:**
   ```bash
   # Explore pre-built functions
   ls -la test-suite/functions/
   ```

3. **Create your own functions:**
   - Build a real API
   - Process real data
   - Integrate with databases

4. **Contribute:**
   - Help improve Java support
   - Add more language runtimes
   - Build the visual debugger!

---

## 💡 Tips for New Users

- **Start simple:** Test hello-world first
- **Check metrics:** Pay attention to cold vs warm starts
- **Watch logs:** Keep server terminal visible
- **Experiment:** Try different event payloads
- **Compare:** Run same function multiple times to see warm start benefits

---

**Welcome to NanoLambda! 🚀**
