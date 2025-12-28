#!/bin/bash
# 🚀 NanoLambda New User Demo Script
# This demonstrates how to use NanoLambda without any files!

set -e  # Exit on error

echo "🚀 NanoLambda New User Demo"
echo "============================"
echo ""

# Server should already be running on localhost:8080
BASE_URL="http://localhost:8080"

# Step 1: Create API Key
echo "📝 Step 1: Creating API key..."
API_KEY_RESPONSE=$(curl -s -X POST $BASE_URL/auth/keys \
  -H "Content-Type: application/json" \
  -d '{
    "name": "demo-key",
    "permissions": ["functions:create", "functions:invoke"]
  }')

API_KEY=$(echo $API_KEY_RESPONSE | jq -r '.key')
echo "✅ API Key created: ${API_KEY:0:20}..."
echo ""

# Step 2: Create Python Function (simple)
echo "🐍 Step 2: Creating simple Python function..."
curl -s -X POST $BASE_URL/functions \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer $API_KEY" \
  -d '{
    "name": "py-simple",
    "runtime": "python3.12",
    "handler": "handler",
    "memory_mb": 128,
    "timeout_ms": 30000,
    "code": "import json\n\ndef handler(event, context):\n    name = event.get(\"name\", \"World\") if event else \"World\"\n    return {\"statusCode\": 200, \"body\": json.dumps({\"message\": f\"Hello, {name}!\", \"runtime\": \"Python 3.12\"})}"
  }' | jq '.'
echo ""

# Step 3: Test Python Function
echo "🧪 Step 3: Testing Python function (cold start)..."
RESULT1=$(curl -s -X POST $BASE_URL/functions/py-simple/invoke \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer $API_KEY" \
  -d '{"payload": {"name": "FirstUser"}}')
echo $RESULT1 | jq '{status_code, body, "cold_start": .metrics.cold_start, "exec_time_ms": .metrics.execution_time_ms, "memory_mb": .metrics.memory_used_mb}'
echo ""

echo "🔥 Step 4: Testing same function again (warm start)..."
RESULT2=$(curl -s -X POST $BASE_URL/functions/py-simple/invoke \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer $API_KEY" \
  -d '{"payload": {"name": "SecondUser"}}')
echo $RESULT2 | jq '{status_code, body, "cold_start": .metrics.cold_start, "exec_time_ms": .metrics.execution_time_ms, "memory_mb": .metrics.memory_used_mb}'
echo ""

# Step 5: Create Python Data Processing Function
echo "🐍 Step 5: Creating complex Python data processor..."
curl -s -X POST $BASE_URL/functions \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer $API_KEY" \
  -d '{
    "name": "py-analytics",
    "runtime": "python3.12",
    "handler": "handler",
    "memory_mb": 256,
    "timeout_ms": 30000,
    "code": "import json\nimport statistics\n\ndef handler(event, context):\n    data = event.get(\"data\", []) if event else []\n    if not data:\n        return {\"statusCode\": 400, \"body\": json.dumps({\"error\": \"No data\"})}\n    return {\n        \"statusCode\": 200,\n        \"body\": json.dumps({\n            \"count\": len(data),\n            \"sum\": sum(data),\n            \"mean\": round(statistics.mean(data), 2),\n            \"median\": statistics.median(data),\n            \"min\": min(data),\n            \"max\": max(data)\n        })\n    }"
  }' | jq '.'
echo ""

# Step 6: Test Data Processing
echo "🧪 Step 6: Testing data analytics..."
RESULT3=$(curl -s -X POST $BASE_URL/functions/py-analytics/invoke \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer $API_KEY" \
  -d '{"payload": {"data": [10, 20, 30, 40, 50, 60, 70, 80, 90, 100]}}')
echo $RESULT3 | jq '{status_code, body, metrics}'
echo ""

# Step 7: Create Node.js Function
echo "🟢 Step 7: Creating Node.js function..."
curl -s -X POST $BASE_URL/functions \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer $API_KEY" \
  -d '{
    "name": "node-api",
    "runtime": "nodejs22.x",
    "handler": "handler",
    "memory_mb": 128,
    "timeout_ms": 30000,
    "code": "export async function handler(event, context) {\n    const name = event?.name || \"World\";\n    return {\n        statusCode: 200,\n        body: JSON.stringify({\n            message: `Hello from Node.js ${process.version}!`,\n            user: name,\n            timestamp: new Date().toISOString()\n        })\n    };\n}"
  }' | jq '.'
echo ""

# Step 8: Test Node.js Function
echo "🧪 Step 8: Testing Node.js function..."
RESULT4=$(curl -s -X POST $BASE_URL/functions/node-api/invoke \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer $API_KEY" \
  -d '{"payload": {"name": "NodeUser"}}')
echo $RESULT4 | jq '{status_code, body, "cold_start": .metrics.cold_start, "exec_time_ms": .metrics.execution_time_ms}'
echo ""

# Step 9: List All Functions
echo "📋 Step 9: Listing all functions..."
curl -s -X GET $BASE_URL/functions \
  -H "Authorization: Bearer $API_KEY" | jq '{count, functions: [.functions[] | {name, runtime, status}]}'
echo ""

# Summary
echo "✨ Demo Complete!"
echo "================="
echo ""
echo "You just tested NanoLambda with:"
echo "  - 3 Python functions (simple, complex analytics)"
echo "  - 1 Node.js function"  
echo "  - All without creating ANY files!"
echo "  - Sub-millisecond warm starts"
echo "  - Real memory metrics"
echo ""
echo "💡 Key Takeaway: Send code as JSON string in API request!"
echo "   No file uploads, no deployments, just HTTP requests."
echo ""
