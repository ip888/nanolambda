#!/bin/bash
# Test script for dashboard functionality

echo "🚀 Starting NanoLambda server..."
cargo run --bin nanolambda-server -- --port 8080 &
SERVER_PID=$!

# Wait for server to start
sleep 6

echo "✅ Server started (PID: $SERVER_PID)"

# Create API key
echo "🔑 Creating API key..."
API_KEY=$(curl -s -X POST http://localhost:8080/auth/keys \
  -H "Content-Type: application/json" \
  -d '{"name":"dashboard-test"}' | jq -r '.key')

if [ -z "$API_KEY" ] || [ "$API_KEY" == "null" ]; then
  echo "❌ Failed to create API key"
  kill $SERVER_PID
  exit 1
fi

echo "✅ API key created: ${API_KEY:0:25}..."

# Create test functions with correct handler format
echo "📦 Creating test functions..."

# Function 1: Simple hello
curl -s -X POST http://localhost:8080/functions \
  -H "Authorization: Bearer $API_KEY" \
  -H "Content-Type: application/json" \
  -d '{
    "name": "dashboard-hello",
    "runtime": "python3.12",
    "handler": "handler",
    "code": "def handler(event, context):\n    return {\"message\": \"Hello!\", \"event\": event}",
    "memory_mb": 128,
    "timeout_ms": 5000
  }' > /dev/null

echo "  ✓ Created dashboard-hello"

# Function 2: Math calculator
curl -s -X POST http://localhost:8080/functions \
  -H "Authorization: Bearer $API_KEY" \
  -H "Content-Type: application/json" \
  -d '{
    "name": "dashboard-calc",
    "runtime": "python3.12",
    "handler": "calculate",
    "code": "def calculate(event, context):\n    a = event.get(\"a\", 0)\n    b = event.get(\"b\", 0)\n    return {\"sum\": a + b, \"product\": a * b}",
    "memory_mb": 128,
    "timeout_ms": 5000
  }' > /dev/null

echo "  ✓ Created dashboard-calc"

# Invoke functions to generate diverse metrics
echo "🔥 Generating metrics (50 invocations)..."

# Mix of invocations
for i in {1..25}; do
  curl -s -X POST http://localhost:8080/functions/dashboard-hello/invoke \
    -H "Authorization: Bearer $API_KEY" \
    -H "Content-Type: application/json" \
    -d "{\"iteration\": $i}" > /dev/null
  echo -n "."
  
  if [ $((i % 2)) -eq 0 ]; then
    curl -s -X POST http://localhost:8080/functions/dashboard-calc/invoke \
      -H "Authorization: Bearer $API_KEY" \
      -H "Content-Type: application/json" \
      -d "{\"a\": $i, \"b\": $((i*2))}" > /dev/null
    echo -n "."
  fi
  
  sleep 0.1
done
echo ""

echo "✅ Invocations completed!"

# Wait a moment for metrics to settle
sleep 2

# Fetch and display metrics
echo ""
echo "📊 Current Metrics:"
echo "=================="
curl -s http://localhost:8080/metrics | jq '.last_hour | {
  total_invocations,
  avg_latency_ms,
  p50_latency_ms,
  p95_latency_ms,
  p99_latency_ms,
  cold_starts,
  cold_start_rate,
  errors,
  error_rate,
  invocations_per_second
}'

echo ""
echo "🌐 Dashboard: http://localhost:8080/dashboard"
echo "📊 Metrics API: http://localhost:8080/metrics"
echo ""
echo "Press Ctrl+C to stop the server (PID: $SERVER_PID)"
echo ""

# Wait for user to stop
wait $SERVER_PID
