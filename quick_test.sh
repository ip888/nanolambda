#!/bin/bash
# Quick server test - checks if server is working and creates API key

echo "🔍 Checking server status..."

# Check if server is running
if ! ps aux | grep "nanolambda-server" | grep -v grep > /dev/null; then
    echo "❌ Server is not running!"
    echo ""
    echo "Start it with:"
    echo "  cargo run --bin nanolambda-server -- --port 8080"
    exit 1
fi

echo "✅ Server process is running"

# Check health endpoint
echo "🏥 Checking health endpoint..."
HEALTH=$(curl -s http://localhost:8080/health)
if [ $? -eq 0 ]; then
    echo "✅ Health check passed: $HEALTH"
else
    echo "❌ Health check failed - server not responding"
    exit 1
fi

# Try to create API key
echo ""
echo "🔑 Testing API key creation..."
RESPONSE=$(curl -s -X POST http://localhost:8080/auth/keys \
  -H "Content-Type: application/json" \
  -d '{"name":"quick-test"}')

API_KEY=$(echo "$RESPONSE" | jq -r '.key')

if [ -z "$API_KEY" ] || [ "$API_KEY" == "null" ]; then
    echo "❌ Failed to create API key"
    echo "Response: $RESPONSE"
    exit 1
fi

echo "✅ API key created successfully!"
echo ""
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo "Your API Key:"
echo "$API_KEY"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo ""
echo "🌐 Dashboard URL: http://localhost:8080/dashboard"
echo ""
echo "Paste the API key into the dashboard input field to see metrics."
