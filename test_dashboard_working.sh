#!/bin/bash
set -e

echo "🧹 Cleaning up old processes..."
pkill -f nanolambda-server 2>/dev/null || true
sleep 2

echo "🚀 Starting NanoLambda server..."
cd /workspaces/nanolambda
cargo run --bin nanolambda-server -- --port 8080 > /tmp/nanolambda.log 2>&1 &
SERVER_PID=$!
echo "Server PID: $SERVER_PID"

# Wait for server to start with health check
echo "⏳ Waiting for server to start..."
MAX_ATTEMPTS=30
ATTEMPT=0
while [ $ATTEMPT -lt $MAX_ATTEMPTS ]; do
  if curl -s http://localhost:8080/health > /dev/null 2>&1; then
    echo "✅ Server is healthy!"
    break
  fi
  ATTEMPT=$((ATTEMPT + 1))
  echo -n "."
  sleep 1
done

if [ $ATTEMPT -eq $MAX_ATTEMPTS ]; then
  echo ""
  echo "❌ Server failed to start after 30 seconds"
  echo "Server logs:"
  tail -20 /tmp/nanolambda.log
  kill $SERVER_PID 2>/dev/null
  exit 1
fi

echo ""

# Create API key
echo "🔑 Creating API key..."
API_KEY=$(curl -s -X POST http://localhost:8080/auth/keys \
  -H "Content-Type: application/json" \
  -d '{"name":"dashboard-test"}' | jq -r '.key')

if [ -z "$API_KEY" ] || [ "$API_KEY" == "null" ]; then
  echo "❌ Failed to create API key"
  echo "Response was: $(curl -s -X POST http://localhost:8080/auth/keys -H "Content-Type: application/json" -d '{"name":"dashboard-test"}')"
  echo ""
  echo "Server logs:"
  tail -20 /tmp/nanolambda.log
  kill $SERVER_PID 2>/dev/null
  exit 1
fi

echo "✅ API key: ${API_KEY:0:20}..."

# Create a working Python function
echo "📦 Creating test function..."
curl -s -X POST http://localhost:8080/functions \
  -H "Authorization: Bearer $API_KEY" \
  -H "Content-Type: application/json" \
  -d '{
    "name": "dashboard-demo",
    "runtime": "python3.12",
    "handler": "main",
    "code": "def main(event, context):\n    return {\n        \"statusCode\": 200,\n        \"body\": \"Success from dashboard demo!\",\n        \"event\": event\n    }",
    "memory_mb": 128,
    "timeout_ms": 5000
  }' > /dev/null

echo "✅ Function created"

# Invoke successfully 25 times
echo "🔥 Invoking function 25 times..."
SUCCESS=0
ERRORS=0

for i in {1..25}; do
  RESPONSE=$(curl -s -X POST http://localhost:8080/functions/dashboard-demo/invoke \
    -H "Authorization: Bearer $API_KEY" \
    -H "Content-Type: application/json" \
    -d "{\"iteration\": $i, \"timestamp\": $(date +%s)}")
  
  STATUS=$(echo "$RESPONSE" | jq -r '.status_code // 500')
  
  if [ "$STATUS" == "200" ]; then
    echo -n "✓"
    SUCCESS=$((SUCCESS + 1))
  else
    echo -n "✗"
    ERRORS=$((ERRORS + 1))
  fi
  
  sleep 0.1
done

echo ""
echo ""
echo "📊 Invocation Results:"
echo "  ✅ Successful: $SUCCESS"
echo "  ❌ Errors: $ERRORS"

# Fetch and display metrics
echo ""
echo "📈 Current Metrics:"
curl -s http://localhost:8080/metrics | jq '.last_hour | {
  total_invocations,
  successful: (.total_invocations - .errors),
  errors,
  error_rate: (.error_rate * 100 | round / 100),
  avg_latency_ms,
  p99_latency_ms,
  cold_starts,
  cold_start_rate: (.cold_start_rate * 100 | round / 100),
  invocations_per_second
}'

echo ""
echo "🌐 Dashboard URL: http://localhost:8080/dashboard"
echo "📊 Metrics API: http://localhost:8080/metrics"
echo ""
echo "✨ Dashboard features:"
echo "  • Real-time charts (auto-refresh every 5 seconds)"
echo "  • Time window switching (Last Hour / 24H / All Time)"
echo "  • 4 professional Chart.js visualizations"
echo "  • Dark theme with status indicators"
echo ""
echo "Press Ctrl+C to stop the server (PID: $SERVER_PID)"
echo ""

# Keep running
wait $SERVER_PID
