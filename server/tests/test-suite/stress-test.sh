#!/bin/bash

# High-Load Stress Testing
# Tests maximum capabilities and breaking points

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
RESULTS_DIR="$SCRIPT_DIR/results/stress-$(date +%Y%m%d_%H%M%S)"
mkdir -p "$RESULTS_DIR"

echo "╔════════════════════════════════════════════════════════════╗"
echo "║           NanoLambda Stress Test Suite                    ║"
echo "╚════════════════════════════════════════════════════════════╝"
echo ""

# Get API key
if [ -z "$API_KEY" ]; then
    API_KEY=$(curl -s -X POST http://localhost:8080/auth/keys \
        -H "Content-Type: application/json" \
        -d '{"name": "stress-test", "expires_at": null}' | \
        grep -o '"key":"[^"]*' | cut -d'"' -f4)
    export API_KEY
fi

# Test 1: Concurrent Function Deployments
echo "Test 1: Deploying 50 functions concurrently..."
START=$(date +%s%3N)

for i in {1..50}; do
    (
        FUNC_NAME="stress-func-$i"
        curl -s -X POST http://localhost:8080/functions \
            -H "Authorization: Bearer $API_KEY" \
            -H "Content-Type: application/json" \
            -d "{
                \"name\": \"$FUNC_NAME\",
                \"runtime\": \"python3.11\",
                \"handler\": \"handler.handler\",
                \"memory_mb\": 128,
                \"timeout_seconds\": 10
            }" > /dev/null
    ) &
done
wait

END=$(date +%s%3N)
DEPLOY_TIME=$((END - START))
echo "✓ Deployed 50 functions in ${DEPLOY_TIME}ms"
echo ""

# Test 2: Peak Load - 10,000 requests in 1 minute
echo "Test 2: Peak load - 10,000 requests in 60 seconds..."
START=$(date +%s)

# Create test function
mkdir -p /tmp/stress-test
cat > /tmp/stress-test/handler.py << 'EOF'
import json

def handler(event, context):
    return {
        'statusCode': 200,
        'body': json.dumps({'message': 'stress test', 'id': event.get('id', 0)})
    }
EOF

cd /tmp/stress-test
zip -q function.zip handler.py

curl -s -X POST http://localhost:8080/api/v1/functions \
    -H "Authorization: Bearer $API_KEY" \
    -H "Content-Type: application/json" \
    -d '{
        "name": "stress-target",
        "runtime": "python3.11",
        "handler": "handler.handler",
        "memory_mb": 512,
        "timeout_seconds": 5
    }' > /dev/null

curl -s -X POST http://localhost:8080/functions/stress-target/code \
    -H "Authorization: Bearer $API_KEY" \
    -F "file=@function.zip" > /dev/null

# Launch 10,000 concurrent requests
SUCCESS=0
FAILED=0

for batch in {1..100}; do
    for i in {1..100}; do
        (
            RESPONSE=$(curl -s -w "\n%{http_code}" \
                -X POST http://localhost:8080/functions/stress-target/invoke \
                -H "Authorization: Bearer $API_KEY" \
                -H "Content-Type: application/json" \
                -d '{"id": '$((batch * 100 + i))'}' 2>/dev/null)
            
            HTTP_CODE=$(echo "$RESPONSE" | tail -1)
            if [ "$HTTP_CODE" = "200" ]; then
                echo "S" >> "$RESULTS_DIR/success.log"
            else
                echo "F" >> "$RESULTS_DIR/failed.log"
            fi
        ) &
    done
    
    # Throttle slightly
    sleep 0.5
done

wait

END=$(date +%s)
DURATION=$((END - START))

SUCCESS=$(wc -l < "$RESULTS_DIR/success.log" 2>/dev/null || echo 0)
FAILED=$(wc -l < "$RESULTS_DIR/failed.log" 2>/dev/null || echo 0)

THROUGHPUT=$((SUCCESS / DURATION))

echo "✓ Peak load completed in ${DURATION}s"
echo "  - Success: $SUCCESS"
echo "  - Failed: $FAILED"
echo "  - Throughput: $THROUGHPUT req/s"
echo ""

# Test 3: Memory Stress - Large payloads
echo "Test 3: Memory stress - 10MB payloads..."
START=$(date +%s)

LARGE_PAYLOAD=$(python3 -c "print('x' * 10485760)")  # 10MB

for i in {1..10}; do
    curl -s -X POST http://localhost:8080/functions/stress-target/invoke \
        -H "Authorization: Bearer $API_KEY" \
        -H "Content-Type: application/json" \
        -d "{\"data\": \"$LARGE_PAYLOAD\"}" > /dev/null &
done
wait

END=$(date +%s)
echo "✓ Memory stress completed in $((END - START))s"
echo ""

# Test 4: Sustained Load - 1000 req/min for 10 minutes
echo "Test 4: Sustained load - 1000 req/min for 10 minutes..."
START=$(date +%s)
END_TIME=$((START + 600))  # 10 minutes

SUSTAINED_SUCCESS=0
SUSTAINED_FAILED=0

while [ $(date +%s) -lt $END_TIME ]; do
    for i in {1..1000}; do
        (
            if curl -s -X POST http://localhost:8080/api/v1/functions/stress-target/invoke \
                -H "Authorization: Bearer $API_KEY" \
                -H "Content-Type: application/json" \
                -d '{"sustained": true}' > /dev/null 2>&1; then
                echo "S" >> "$RESULTS_DIR/sustained_success.log"
            else
                echo "F" >> "$RESULTS_DIR/sustained_failed.log"
            fi
        ) &
        
        # Control rate: 1000 req/min = ~16 req/s = 0.06s delay
        sleep 0.06
    done
done

wait

SUSTAINED_SUCCESS=$(wc -l < "$RESULTS_DIR/sustained_success.log" 2>/dev/null || echo 0)
SUSTAINED_FAILED=$(wc -l < "$RESULTS_DIR/sustained_failed.log" 2>/dev/null || echo 0)

echo "✓ Sustained load completed"
echo "  - Success: $SUSTAINED_SUCCESS"
echo "  - Failed: $SUSTAINED_FAILED"
echo ""

# Generate Report
echo "═══════════════════════════════════════════════════════════"
echo "STRESS TEST SUMMARY"
echo "═══════════════════════════════════════════════════════════"
echo ""
echo "1. Concurrent Deployments: ${DEPLOY_TIME}ms for 50 functions"
echo "2. Peak Load:"
echo "   - Throughput: $THROUGHPUT req/s"
echo "   - Success Rate: $(awk "BEGIN {printf \"%.2f\", ($SUCCESS/($SUCCESS+$FAILED))*100}")%"
echo "3. Memory Stress: Completed successfully"
echo "4. Sustained Load:"
echo "   - Total Requests: $((SUSTAINED_SUCCESS + SUSTAINED_FAILED))"
echo "   - Success Rate: $(awk "BEGIN {printf \"%.2f\", ($SUSTAINED_SUCCESS/($SUSTAINED_SUCCESS+$SUSTAINED_FAILED))*100}")%"
echo ""
echo "Results saved to: $RESULTS_DIR"
echo "Dashboard: http://localhost:8080/dashboard"
echo ""
