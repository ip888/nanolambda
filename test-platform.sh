#!/bin/bash

# NanoLambda Platform Test Script
# This script tests all core functionality of the platform

# Don't exit on error - we want to see all test results
set +e

echo "🚀 Testing NanoLambda Platform"
echo "================================"
echo ""

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

# Base URL
BASE_URL="http://localhost:8080"

# Test counter
TESTS_PASSED=0
TESTS_FAILED=0

# Helper function to print test results
print_result() {
    if [ $1 -eq 0 ]; then
        echo -e "${GREEN}✓ PASS${NC}: $2"
        ((TESTS_PASSED++))
    else
        echo -e "${RED}✗ FAIL${NC}: $2"
        ((TESTS_FAILED++))
    fi
}

# Helper function to check JSON response
check_response() {
    local response="$1"
    local expected_field="$2"
    local test_name="$3"
    
    if echo "$response" | grep -q "$expected_field"; then
        print_result 0 "$test_name"
        return 0
    else
        print_result 1 "$test_name"
        echo "Response: $response"
        return 1
    fi
}

echo "1. Testing Health Check..."
RESPONSE=$(curl -s $BASE_URL/health)
check_response "$RESPONSE" "healthy" "Health endpoint"
echo ""

echo "2. Cleaning up old test functions..."
curl -s -X DELETE $BASE_URL/functions/test-python > /dev/null 2>&1 || true
curl -s -X DELETE $BASE_URL/functions/test-nodejs > /dev/null 2>&1 || true
curl -s -X DELETE $BASE_URL/functions/json-processor > /dev/null 2>&1 || true
sleep 0.5  # Give server time to process deletes
echo "Cleanup complete"
echo ""

echo "3. Testing Python Function Creation..."
RESPONSE=$(curl -s -X POST $BASE_URL/functions \
  -H "Content-Type: application/json" \
  -d '{
    "name": "test-python",
    "runtime": "python",
    "handler": "handler",
    "code": "def handler(event, context):\n    return {\"input\": event, \"message\": \"Success\"}",
    "timeout_ms": 5000,
    "memory_mb": 128
  }')
check_response "$RESPONSE" "test-python" "Create Python function"
sleep 0.2  # Give server time to persist
echo ""

echo "4. Testing Python Function Invocation (Cold Start)..."
RESPONSE=$(curl -s -X POST $BASE_URL/functions/test-python/invoke \
  -H "Content-Type: application/json" \
  -d '{"test": "data", "value": 42}')
check_response "$RESPONSE" "request_id" "Python invocation"
check_response "$RESPONSE" "Success" "Python output"

# Extract cold start status
if echo "$RESPONSE" | grep -q '"cold_start":true'; then
    echo -e "${YELLOW}  → Cold start detected${NC}"
elif echo "$RESPONSE" | grep -q '"cold_start":false'; then
    echo -e "${GREEN}  → Warm start (process pooling working!)${NC}"
fi

# Extract execution time
EXEC_TIME=$(echo "$RESPONSE" | grep -o '"execution_time_ms":[0-9]*' | grep -o '[0-9]*')
if [ ! -z "$EXEC_TIME" ]; then
    echo -e "${GREEN}  → Execution time: ${EXEC_TIME}ms${NC}"
fi
echo ""

echo "5. Testing Python Function Invocation (Warm Start)..."
RESPONSE=$(curl -s -X POST $BASE_URL/functions/test-python/invoke \
  -H "Content-Type: application/json" \
  -d '{"test": "warm", "value": 99}')
check_response "$RESPONSE" "request_id" "Python warm invocation"

# Check for warm start
if echo "$RESPONSE" | grep -q '"cold_start":false'; then
    print_result 0 "Process pooling confirmed (warm start)"
else
    print_result 1 "Process pooling NOT working (still cold)"
fi

# Extract execution time
EXEC_TIME=$(echo "$RESPONSE" | grep -o '"execution_time_ms":[0-9]*' | grep -o '[0-9]*')
if [ ! -z "$EXEC_TIME" ]; then
    echo -e "${GREEN}  → Execution time: ${EXEC_TIME}ms (should be 0-1ms if warm!)${NC}"
fi
echo ""

echo "6. Testing Node.js Function Creation..."
RESPONSE=$(curl -s -X POST $BASE_URL/functions \
  -H "Content-Type: application/json" \
  -d '{
    "name": "test-nodejs",
    "runtime": "nodejs",
    "handler": "handler",
    "code": "exports.handler = async (event, context) => { return { input: event, message: \"Node.js works\" }; };",
    "timeout_ms": 5000,
    "memory_mb": 128
  }')
check_response "$RESPONSE" "test-nodejs" "Create Node.js function"
sleep 0.2  # Give server time to persist
echo ""

echo "7. Testing Node.js Function Invocation..."
RESPONSE=$(curl -s -X POST $BASE_URL/functions/test-nodejs/invoke \
  -H "Content-Type: application/json" \
  -d '{"test": "nodejs", "value": 123}')
check_response "$RESPONSE" "request_id" "Node.js invocation"
check_response "$RESPONSE" "Node.js works" "Node.js output"
echo ""

echo "8. Testing Function Listing..."
RESPONSE=$(curl -s $BASE_URL/functions)
check_response "$RESPONSE" "test-python" "List includes Python function"
check_response "$RESPONSE" "test-nodejs" "List includes Node.js function"
check_response "$RESPONSE" '"count":2' "Function count"
echo ""

echo "9. Testing Get Single Function..."
RESPONSE=$(curl -s $BASE_URL/functions/test-python)
check_response "$RESPONSE" "test-python" "Get function details"
check_response "$RESPONSE" "active" "Function status"
echo ""

echo "10. Testing Complex JSON Processing..."
RESPONSE=$(curl -s -X POST $BASE_URL/functions \
  -H "Content-Type: application/json" \
  -d '{
    "name": "json-processor",
    "runtime": "python",
    "handler": "handler",
    "code": "import json\ndef handler(event, context):\n    if event is None:\n        event = {}\n    items = event.get(\"items\", [])\n    total = sum(item.get(\"price\", 0) for item in items)\n    return {\"total\": total, \"count\": len(items)}",
    "timeout_ms": 5000,
    "memory_mb": 128
  }')
check_response "$RESPONSE" "json-processor" "Create JSON processor"
sleep 0.2  # Give server time to persist

RESPONSE=$(curl -s -X POST $BASE_URL/functions/json-processor/invoke \
  -H "Content-Type: application/json" \
  -d '{"items": [{"name": "item1", "price": 10}, {"name": "item2", "price": 20}, {"name": "item3", "price": 30}]}')
check_response "$RESPONSE" '"total":60' "JSON processing - sum"
check_response "$RESPONSE" '"count":3' "JSON processing - count"
echo ""

echo "11. Testing Error Handling (Duplicate Function)..."
RESPONSE=$(curl -s -X POST $BASE_URL/functions \
  -H "Content-Type: application/json" \
  -d '{
    "name": "test-python",
    "runtime": "python",
    "handler": "handler",
    "code": "def handler(event, context): return {}",
    "timeout_ms": 5000,
    "memory_mb": 128
  }')
check_response "$RESPONSE" "already exists" "Duplicate detection"
echo ""

echo "12. Testing Function Update..."
RESPONSE=$(curl -s -X PUT $BASE_URL/functions/test-python \
  -H "Content-Type: application/json" \
  -d '{
    "name": "test-python",
    "runtime": "python",
    "handler": "handler",
    "code": "def handler(event, context):\n    return {\"message\": \"Updated version\", \"data\": event}",
    "timeout_ms": 3000,
    "memory_mb": 256
  }')
check_response "$RESPONSE" "test-python" "Update function"
sleep 0.2  # Give server time to persist
echo ""

echo "13. Testing Updated Function Execution..."
RESPONSE=$(curl -s -X POST $BASE_URL/functions/test-python/invoke \
  -H "Content-Type: application/json" \
  -d '{"test": "update"}')
check_response "$RESPONSE" "Updated version" "Updated code executes"
echo ""

echo "================================"
echo "Test Summary"
echo "================================"
echo -e "${GREEN}Passed: $TESTS_PASSED${NC}"
echo -e "${RED}Failed: $TESTS_FAILED${NC}"
echo ""

if [ $TESTS_FAILED -eq 0 ]; then
    echo -e "${GREEN}🎉 ALL TESTS PASSED! Platform is fully functional!${NC}"
    exit 0
else
    echo -e "${YELLOW}⚠️  Some tests failed. Check the output above.${NC}"
    exit 1
fi
