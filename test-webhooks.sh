#!/bin/bash
# Test Stripe webhook signature verification and event handling

set -e

# Colors for output
GREEN='\033[0;32m'
RED='\033[0;31m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

echo -e "${YELLOW}=== Stripe Webhook Testing Script ===${NC}\n"

# Configuration
API_URL="${API_URL:-http://localhost:8080}"
WEBHOOK_SECRET="${STRIPE_WEBHOOK_SECRET:-whsec_test_secret}"

# Generate test webhook signature
generate_signature() {
    local payload="$1"
    local timestamp=$(date +%s)
    local signed_payload="${timestamp}.${payload}"
    
    # Compute HMAC-SHA256
    local signature=$(echo -n "$signed_payload" | openssl dgst -sha256 -hmac "$WEBHOOK_SECRET" | awk '{print $2}')
    
    echo "t=${timestamp},v1=${signature}"
}

# Test 1: subscription.created event
echo -e "${YELLOW}Test 1: Subscription Created${NC}"
PAYLOAD='{
  "id": "evt_test_1",
  "type": "customer.subscription.created",
  "created": '$(date +%s)',
  "data": {
    "object": {
      "id": "sub_test_123",
      "status": "active",
      "customer": "cus_test_456"
    }
  }
}'

SIGNATURE=$(generate_signature "$PAYLOAD")
echo "Signature: $SIGNATURE"

RESPONSE=$(curl -s -X POST "$API_URL/webhooks/stripe" \
  -H "Content-Type: application/json" \
  -H "Stripe-Signature: $SIGNATURE" \
  -d "$PAYLOAD")

if echo "$RESPONSE" | jq -e '.success' > /dev/null 2>&1; then
    echo -e "${GREEN}✓ Test 1 PASSED${NC}\n"
else
    echo -e "${RED}✗ Test 1 FAILED: $RESPONSE${NC}\n"
fi

# Test 2: subscription.updated event
echo -e "${YELLOW}Test 2: Subscription Updated${NC}"
PAYLOAD='{
  "id": "evt_test_2",
  "type": "customer.subscription.updated",
  "created": '$(date +%s)',
  "data": {
    "object": {
      "id": "sub_test_123",
      "status": "past_due",
      "customer": "cus_test_456"
    }
  }
}'

SIGNATURE=$(generate_signature "$PAYLOAD")
RESPONSE=$(curl -s -X POST "$API_URL/webhooks/stripe" \
  -H "Content-Type: application/json" \
  -H "Stripe-Signature: $SIGNATURE" \
  -d "$PAYLOAD")

if echo "$RESPONSE" | jq -e '.success' > /dev/null 2>&1; then
    echo -e "${GREEN}✓ Test 2 PASSED${NC}\n"
else
    echo -e "${RED}✗ Test 2 FAILED: $RESPONSE${NC}\n"
fi

# Test 3: invalid signature (should fail)
echo -e "${YELLOW}Test 3: Invalid Signature (Expected Failure)${NC}"
PAYLOAD='{
  "id": "evt_test_3",
  "type": "customer.subscription.deleted",
  "created": '$(date +%s)',
  "data": {
    "object": {
      "id": "sub_test_123",
      "status": "canceled",
      "customer": "cus_test_456"
    }
  }
}'

INVALID_SIGNATURE="t=$(date +%s),v1=invalid_signature_here"
RESPONSE=$(curl -s -w "\n%{http_code}" -X POST "$API_URL/webhooks/stripe" \
  -H "Content-Type: application/json" \
  -H "Stripe-Signature: $INVALID_SIGNATURE" \
  -d "$PAYLOAD")

HTTP_CODE=$(echo "$RESPONSE" | tail -1)
if [ "$HTTP_CODE" == "400" ]; then
    echo -e "${GREEN}✓ Test 3 PASSED (correctly rejected)${NC}\n"
else
    echo -e "${RED}✗ Test 3 FAILED: Expected 400, got $HTTP_CODE${NC}\n"
fi

# Test 4: payment succeeded
echo -e "${YELLOW}Test 4: Payment Succeeded${NC}"
PAYLOAD='{
  "id": "evt_test_4",
  "type": "invoice.payment_succeeded",
  "created": '$(date +%s)',
  "data": {
    "object": {
      "id": "in_test_789",
      "customer": "cus_test_456",
      "subscription": "sub_test_123",
      "amount_paid": 99900
    }
  }
}'

SIGNATURE=$(generate_signature "$PAYLOAD")
RESPONSE=$(curl -s -X POST "$API_URL/webhooks/stripe" \
  -H "Content-Type: application/json" \
  -H "Stripe-Signature: $SIGNATURE" \
  -d "$PAYLOAD")

if echo "$RESPONSE" | jq -e '.success' > /dev/null 2>&1; then
    echo -e "${GREEN}✓ Test 4 PASSED${NC}\n"
else
    echo -e "${RED}✗ Test 4 FAILED: $RESPONSE${NC}\n"
fi

# Test 5: payment failed
echo -e "${YELLOW}Test 5: Payment Failed${NC}"
PAYLOAD='{
  "id": "evt_test_5",
  "type": "invoice.payment_failed",
  "created": '$(date +%s)',
  "data": {
    "object": {
      "id": "in_test_790",
      "customer": "cus_test_456",
      "subscription": "sub_test_123",
      "amount_due": 99900
    }
  }
}'

SIGNATURE=$(generate_signature "$PAYLOAD")
RESPONSE=$(curl -s -X POST "$API_URL/webhooks/stripe" \
  -H "Content-Type: application/json" \
  -H "Stripe-Signature: $SIGNATURE" \
  -d "$PAYLOAD")

if echo "$RESPONSE" | jq -e '.success' > /dev/null 2>&1; then
    echo -e "${GREEN}✓ Test 5 PASSED${NC}\n"
else
    echo -e "${RED}✗ Test 5 FAILED: $RESPONSE${NC}\n"
fi

echo -e "${YELLOW}=== All Tests Complete ===${NC}"
