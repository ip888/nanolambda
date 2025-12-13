#!/usr/bin/env python3
"""
Test Stripe webhook signature verification
Generates properly signed webhook payloads
"""

import json
import time
import hmac
import hashlib
import requests
import sys

WEBHOOK_SECRET = "whsec_test_secret"
API_URL = "http://localhost:8080"

def sign_payload(payload_str, secret):
    """Generate Stripe-compatible signature"""
    timestamp = str(int(time.time()))
    signed_payload = f"{timestamp}.{payload_str}"
    
    signature = hmac.new(
        secret.encode('utf-8'),
        signed_payload.encode('utf-8'),
        hashlib.sha256
    ).hexdigest()
    
    return f"t={timestamp},v1={signature}"

def test_webhook(event_type, data):
    """Test a webhook event"""
    payload = {
        "id": f"evt_test_{int(time.time())}",
        "type": event_type,
        "created": int(time.time()),
        "data": {"object": data}
    }
    
    payload_str = json.dumps(payload, separators=(',', ':'))
    signature = sign_payload(payload_str, WEBHOOK_SECRET)
    
    print(f"\n{'='*60}")
    print(f"Testing: {event_type}")
    print(f"{'='*60}")
    print(f"Payload: {payload_str[:100]}...")
    print(f"Signature: {signature}")
    
    response = requests.post(
        f"{API_URL}/webhooks/stripe",
        headers={
            "Content-Type": "application/json",
            "Stripe-Signature": signature
        },
        data=payload_str
    )
    
    print(f"Status: {response.status_code}")
    print(f"Response: {response.text}")
    
    return response.status_code == 200

def main():
    print("🧪 Stripe Webhook Testing Suite")
    print("="*60)
    
    tests_passed = 0
    tests_total = 0
    
    # Test 1: Subscription Created
    tests_total += 1
    if test_webhook("customer.subscription.created", {
        "id": "sub_test_123",
        "status": "active",
        "customer": "cus_test_456"
    }):
        tests_passed += 1
        print("✅ PASSED")
    else:
        print("❌ FAILED")
    
    # Test 2: Subscription Updated
    tests_total += 1
    if test_webhook("customer.subscription.updated", {
        "id": "sub_test_123",
        "status": "past_due",
        "customer": "cus_test_456"
    }):
        tests_passed += 1
        print("✅ PASSED")
    else:
        print("❌ FAILED")
    
    # Test 3: Subscription Deleted
    tests_total += 1
    if test_webhook("customer.subscription.deleted", {
        "id": "sub_test_123",
        "status": "canceled",
        "customer": "cus_test_456"
    }):
        tests_passed += 1
        print("✅ PASSED")
    else:
        print("❌ FAILED")
    
    # Test 4: Payment Succeeded
    tests_total += 1
    if test_webhook("invoice.payment_succeeded", {
        "id": "in_test_789",
        "customer": "cus_test_456",
        "subscription": "sub_test_123",
        "amount_paid": 99900
    }):
        tests_passed += 1
        print("✅ PASSED")
    else:
        print("❌ FAILED")
    
    # Test 5: Payment Failed
    tests_total += 1
    if test_webhook("invoice.payment_failed", {
        "id": "in_test_790",
        "customer": "cus_test_456",
        "subscription": "sub_test_123",
        "amount_due": 99900
    }):
        tests_passed += 1
        print("✅ PASSED")
    else:
        print("❌ FAILED")
    
    # Test 6: Invalid Signature (should fail)
    tests_total += 1
    print(f"\n{'='*60}")
    print("Testing: Invalid Signature (should fail)")
    print(f"{'='*60}")
    
    payload = {"id": "evt_test", "type": "test", "created": int(time.time()), "data": {"object": {}}}
    response = requests.post(
        f"{API_URL}/webhooks/stripe",
        headers={
            "Content-Type": "application/json",
            "Stripe-Signature": "t=123,v1=invalid"
        },
        json=payload
    )
    
    if response.status_code == 400:
        tests_passed += 1
        print("✅ PASSED (correctly rejected)")
    else:
        print(f"❌ FAILED (expected 400, got {response.status_code})")
    
    # Summary
    print(f"\n{'='*60}")
    print(f"Results: {tests_passed}/{tests_total} tests passed")
    print(f"{'='*60}")
    
    return 0 if tests_passed == tests_total else 1

if __name__ == "__main__":
    sys.exit(main())
