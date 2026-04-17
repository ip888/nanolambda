# NanoLambda Real-World Testing Guide

## Overview
This guide provides step-by-step instructions for testing NanoLambda with realistic workloads.

> 🚀 **NEW: Automated Test Suite Available!**
> 
> For production-ready, comprehensive testing with pre-built functions and automated scripts:
> ```bash
> cd /workspaces/nanolambda/test-suite
> ./run-all-tests.sh
> ```
> 
> See [`/test-suite/COMPREHENSIVE_GUIDE.md`](../test-suite/COMPREHENSIVE_GUIDE.md) for:
> - ✅ Ready-to-use functions (Python, Node.js, Java)
> - ✅ Automated test scenarios (sanity, load, stress, 24/7 monitoring)
> - ✅ Dashboard integration with real-time metrics
> - ✅ One-command execution and cleanup
> 
> This guide below shows **manual testing** for learning and customization.

---

## Quick Setup

### Start NanoLambda
```bash
# Start server
./target/release/nanolambda-server &

# Create API key
curl -X POST http://localhost:8080/api/v1/auth/keys \
  -H "Content-Type: application/json" \
  -d '{"name": "test-key", "expires_at": null}'

# Save the returned key
export API_KEY="your-key-here"
```

---

## Test 1: Simple API Function

### Create Function
```bash
mkdir -p /tmp/api-test
cat > /tmp/api-test/handler.py << 'EOF'
import json
import time

def handler(event, context):
    # Simulate processing time
    time.sleep(0.05)
    
    body = json.loads(event.get('body', '{}'))
    return {
        'statusCode': 200,
        'body': json.dumps({
            'message': 'Hello ' + body.get('name', 'World'),
            'timestamp': int(time.time())
        })
    }
EOF
```

### Deploy and Test
```bash
# Create function
curl -X POST http://localhost:8080/api/v1/functions \
  -H "Authorization: Bearer $API_KEY" \
  -H "Content-Type: application/json" \
  -d '{
    "name": "hello-api",
    "runtime": "python3.11",
    "handler": "handler.handler",
    "memory_mb": 128,
    "timeout_seconds": 30
  }'

# Upload code
cd /tmp/api-test && zip -r code.zip .
curl -X POST http://localhost:8080/api/v1/functions/hello-api/code \
  -H "Authorization: Bearer $API_KEY" \
  -F "file=@code.zip"

# Test cold start
curl -X POST http://localhost:8080/api/v1/functions/hello-api/invoke \
  -H "Authorization: Bearer $API_KEY" \
  -H "Content-Type: application/json" \
  -d '{"name": "Alice"}'

# Test warm start (immediate second call)
curl -X POST http://localhost:8080/api/v1/functions/hello-api/invoke \
  -H "Authorization: Bearer $API_KEY" \
  -H "Content-Type: application/json" \
  -d '{"name": "Bob"}'
```

**Expected Results:**
- Cold start: ~60-80ms
- Warm start: ~50-55ms

---

## Test 2: Load Testing

### Simple Load Test
```bash
# Create test payload
echo '{"name": "LoadTest"}' > /tmp/payload.json

# Run 100 requests with 10 concurrent
for i in {1..100}; do
  curl -X POST http://localhost:8080/api/v1/functions/hello-api/invoke \
    -H "Authorization: Bearer $API_KEY" \
    -H "Content-Type: application/json" \
    -d @/tmp/payload.json &
  
  # Control concurrency
  if [ $((i % 10)) -eq 0 ]; then wait; fi
done
wait
```

### Check Results
Visit the dashboard at `http://localhost:8080/dashboard` to see:
- Total invocations: 100+
- Average latency: ~50-60ms
- Success rate: ~100%

---

## Test 3: Data Processing

### Create Data Processor
```bash
mkdir -p /tmp/data-test
cat > /tmp/data-test/handler.py << 'EOF'
import json

def handler(event, context):
    numbers = event.get('numbers', [])
    
    if not numbers:
        return {'error': 'No numbers provided'}
    
    result = {
        'count': len(numbers),
        'sum': sum(numbers),
        'average': sum(numbers) / len(numbers),
        'min': min(numbers),
        'max': max(numbers)
    }
    
    return {'result': result}
EOF

# Deploy
curl -X POST http://localhost:8080/api/v1/functions \
  -H "Authorization: Bearer $API_KEY" \
  -H "Content-Type: application/json" \
  -d '{
    "name": "data-processor",
    "runtime": "python3.11", 
    "handler": "handler.handler",
    "memory_mb": 256,
    "timeout_seconds": 30
  }'

cd /tmp/data-test && zip -r code.zip .
curl -X POST http://localhost:8080/api/v1/functions/data-processor/code \
  -H "Authorization: Bearer $API_KEY" \
  -F "file=@code.zip"

# Test with sample data
curl -X POST http://localhost:8080/api/v1/functions/data-processor/invoke \
  -H "Authorization: Bearer $API_KEY" \
  -H "Content-Type: application/json" \
  -d '{"numbers": [10, 20, 30, 40, 50]}'
```

---

## Test 4: Error Handling

### Test Function Errors
```bash
# Test with invalid data
curl -X POST http://localhost:8080/api/v1/functions/data-processor/invoke \
  -H "Authorization: Bearer $API_KEY" \
  -H "Content-Type: application/json" \
  -d '{"invalid": "data"}'

# Test timeout (create slow function)
mkdir -p /tmp/slow-test
cat > /tmp/slow-test/handler.py << 'EOF'
import time

def handler(event, context):
    # Sleep longer than timeout
    time.sleep(35)
    return {'message': 'This should timeout'}
EOF

# Deploy with 30s timeout
curl -X POST http://localhost:8080/api/v1/functions \
  -H "Authorization: Bearer $API_KEY" \
  -H "Content-Type: application/json" \
  -d '{
    "name": "slow-function",
    "runtime": "python3.11",
    "handler": "handler.handler", 
    "memory_mb": 128,
    "timeout_seconds": 30
  }'

cd /tmp/slow-test && zip -r code.zip .
curl -X POST http://localhost:8080/api/v1/functions/slow-function/code \
  -H "Authorization: Bearer $API_KEY" \
  -F "file=@code.zip"

# This should timeout after 30s
curl -X POST http://localhost:8080/api/v1/functions/slow-function/invoke \
  -H "Authorization: Bearer $API_KEY" \
  -H "Content-Type: application/json" \
  -d '{}'
```

---

## What to Expect

### Performance Benchmarks
| Scenario | Expected Latency | Notes |
|----------|-----------------|-------|
| Cold start | 60-100ms | First invocation |
| Warm start | 50-60ms | Subsequent calls |
| Simple processing | +10-50ms | Based on function logic |
| Heavy processing | +100-500ms | Data processing, etc. |

### Dashboard Metrics
After testing, your dashboard should show:
- **Active Functions**: 3-4 functions
- **Total Invocations**: 150+ requests
- **Success Rate**: >95%
- **Average Latency**: 50-80ms range

---

## Troubleshooting

### Common Issues

**Functions not responding:**
```bash
# Check if server is running
curl http://localhost:8080/health

# List functions
curl -H "Authorization: Bearer $API_KEY" \
  http://localhost:8080/api/v1/functions
```

**High latency:**
- First call to a function is always slower (cold start)
- Check function code for expensive operations
- Increase memory if processing large data

**Dashboard shows no data:**
- Run at least one function invocation
- Refresh the dashboard page
- Check browser console for errors

### Server Logs
```bash
# Check server output for errors
pkill -f nanolambda-server
./target/release/nanolambda-server
```

---

## Next Steps

1. **Try More Complex Functions**: Add dependencies, external API calls
2. **Monitor Performance**: Use the dashboard to track metrics
3. **Production Setup**: See deployment guides for scaling

For more advanced testing scenarios, check the full documentation.
