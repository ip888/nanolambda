#!/bin/bash
# NanoLambda Edge Platform - Local Test Suite
# Requires: wrangler dev server running on localhost:8787

set -e

BASE_URL="${BASE_URL:-http://localhost:8787}"
API_KEY="${API_KEY:-dev-test-key}"
PASSED=0
FAILED=0

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

echo "=============================================="
echo "  NanoLambda Edge Platform - Test Suite"
echo "=============================================="
echo "Base URL: $BASE_URL"
echo "API Key: $API_KEY"
echo ""

# Helper functions
test_endpoint() {
    local name=$1
    local method=$2
    local endpoint=$3
    local data=$4
    local expected_status=$5
    
    echo -n "Testing $name... "
    
    if [ "$method" = "GET" ]; then
        response=$(curl -s -w "\n%{http_code}" "$BASE_URL$endpoint" \
            -H "X-Api-Key: $API_KEY")
    else
        response=$(curl -s -w "\n%{http_code}" -X "$method" "$BASE_URL$endpoint" \
            -H "Content-Type: application/json" \
            -H "X-Api-Key: $API_KEY" \
            -d "$data")
    fi
    
    status=$(echo "$response" | tail -n1)
    body=$(echo "$response" | sed '$d')
    
    if [ "$status" = "$expected_status" ]; then
        echo -e "${GREEN}PASSED${NC} (HTTP $status)"
        ((PASSED++))
        if [ -n "$body" ]; then
            echo "$body" | jq . 2>/dev/null || echo "$body"
        fi
    else
        echo -e "${RED}FAILED${NC} (Expected HTTP $expected_status, got $status)"
        ((FAILED++))
        if [ -n "$body" ]; then
            echo "$body" | jq . 2>/dev/null || echo "$body"
        fi
    fi
    echo ""
}

# ============================================
# Health Check Tests
# ============================================
echo -e "${YELLOW}=== Health Check ===${NC}"
test_endpoint "Health check" "GET" "/health" "" "200"

# ============================================
# Function Management Tests
# ============================================
echo -e "${YELLOW}=== Function Management ===${NC}"

# Create a function
FUNC_DATA='{
    "name": "test-function",
    "runtime": "javascript",
    "code": "export default { async fetch(req) { const data = await req.json(); return new Response(JSON.stringify({greeting: \"Hello \" + (data.name || \"World\")}), {headers: {\"Content-Type\": \"application/json\"}}); } }"
}'
test_endpoint "Create function" "POST" "/v1/functions" "$FUNC_DATA" "200"

# List functions
test_endpoint "List functions" "GET" "/v1/functions" "" "200"

# Get the function ID from the list
FUNC_ID=$(curl -s "$BASE_URL/v1/functions" -H "X-Api-Key: $API_KEY" | jq -r '.functions[0].id // empty')

if [ -n "$FUNC_ID" ]; then
    echo "Using function ID: $FUNC_ID"
    
    # Get single function
    test_endpoint "Get function by ID" "GET" "/v1/functions/$FUNC_ID" "" "200"
    
    # Update function
    UPDATE_DATA='{"code": "export default { fetch() { return new Response(\"Updated!\"); } }"}'
    test_endpoint "Update function" "PUT" "/v1/functions/$FUNC_ID" "$UPDATE_DATA" "200"
    
    # Note: Function invocation requires JavaScript runtime which is a placeholder
    echo -e "${YELLOW}Skipping function invocation (requires JS runtime)${NC}"
    echo ""
fi

# ============================================
# Vector Operations Tests
# ============================================
echo -e "${YELLOW}=== Vector Operations (QuartzDB) ===${NC}"

# Upsert vectors
VECTORS_DATA='{
    "namespace": "test-vectors",
    "vectors": [
        {"id": "doc1", "values": [0.1, 0.2, 0.3, 0.4, 0.5], "metadata": {"title": "First Document", "category": "tech"}},
        {"id": "doc2", "values": [0.2, 0.3, 0.4, 0.5, 0.6], "metadata": {"title": "Second Document", "category": "science"}},
        {"id": "doc3", "values": [0.9, 0.8, 0.7, 0.6, 0.5], "metadata": {"title": "Third Document", "category": "tech"}}
    ]
}'
test_endpoint "Upsert vectors" "POST" "/v1/vectors/upsert" "$VECTORS_DATA" "200"

# Query vectors
QUERY_DATA='{
    "namespace": "test-vectors",
    "vector": [0.15, 0.25, 0.35, 0.45, 0.55],
    "top_k": 3,
    "include_metadata": true
}'
test_endpoint "Query vectors" "POST" "/v1/vectors/query" "$QUERY_DATA" "200"

# Query with filter (if supported)
QUERY_FILTER_DATA='{
    "namespace": "test-vectors",
    "vector": [0.1, 0.2, 0.3, 0.4, 0.5],
    "top_k": 2,
    "include_metadata": true,
    "filter": {"category": "tech"}
}'
test_endpoint "Query with filter" "POST" "/v1/vectors/query" "$QUERY_FILTER_DATA" "200"

# Delete vectors
DELETE_DATA='{"namespace": "test-vectors", "ids": ["doc3"]}'
test_endpoint "Delete vectors" "POST" "/v1/vectors/delete" "$DELETE_DATA" "200"

# Get stats
test_endpoint "Vector stats" "GET" "/v1/vectors/stats" "" "200"

# ============================================
# AI Endpoints Tests (require Cloudflare auth)
# ============================================
echo -e "${YELLOW}=== AI Endpoints (May fail in local mode) ===${NC}"

EMBED_DATA='{"input": "Hello, world!", "model": "@cf/baai/bge-small-en-v1.5"}'
echo -n "Testing AI embeddings... "
embed_response=$(curl -s -w "\n%{http_code}" -X POST "$BASE_URL/v1/ai/embeddings" \
    -H "Content-Type: application/json" \
    -H "X-Api-Key: $API_KEY" \
    -d "$EMBED_DATA")
embed_status=$(echo "$embed_response" | tail -n1)
if [ "$embed_status" = "200" ]; then
    echo -e "${GREEN}PASSED${NC} (AI available)"
    ((PASSED++))
else
    echo -e "${YELLOW}SKIPPED${NC} (AI requires Cloudflare auth in local mode)"
fi
echo ""

# ============================================
# Authentication Tests
# ============================================
echo -e "${YELLOW}=== Authentication Tests ===${NC}"

# Test with invalid API key (should fail)
echo -n "Testing invalid API key... "
invalid_response=$(curl -s -w "\n%{http_code}" "$BASE_URL/v1/functions" -H "X-Api-Key: invalid-key")
invalid_status=$(echo "$invalid_response" | tail -n1)
if [ "$invalid_status" = "401" ]; then
    echo -e "${GREEN}PASSED${NC} (Rejected invalid key)"
    ((PASSED++))
else
    echo -e "${RED}FAILED${NC} (Expected 401, got $invalid_status)"
    ((FAILED++))
fi
echo ""

# Test without API key (should fail)
echo -n "Testing no API key... "
no_key_response=$(curl -s -w "\n%{http_code}" "$BASE_URL/v1/functions")
no_key_status=$(echo "$no_key_response" | tail -n1)
if [ "$no_key_status" = "401" ]; then
    echo -e "${GREEN}PASSED${NC} (Rejected missing key)"
    ((PASSED++))
else
    echo -e "${RED}FAILED${NC} (Expected 401, got $no_key_status)"
    ((FAILED++))
fi
echo ""

# ============================================
# Cleanup
# ============================================
echo -e "${YELLOW}=== Cleanup ===${NC}"

if [ -n "$FUNC_ID" ]; then
    test_endpoint "Delete function" "DELETE" "/v1/functions/$FUNC_ID" "" "200"
fi

# Delete all test vectors
DELETE_ALL='{
    "namespace": "test-vectors",
    "ids": ["doc1", "doc2"]
}'
test_endpoint "Cleanup vectors" "POST" "/v1/vectors/delete" "$DELETE_ALL" "200"

# ============================================
# Summary
# ============================================
echo "=============================================="
echo "  Test Summary"
echo "=============================================="
echo -e "Passed: ${GREEN}$PASSED${NC}"
echo -e "Failed: ${RED}$FAILED${NC}"
echo ""

if [ $FAILED -eq 0 ]; then
    echo -e "${GREEN}All tests passed!${NC}"
    exit 0
else
    echo -e "${RED}Some tests failed.${NC}"
    exit 1
fi
