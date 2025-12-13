#!/usr/bin/env python3
"""
Test script for email notification system.
Tests email sending for various payment events.

NOTE: This requires SMTP credentials to be set in environment variables.
For testing, you can use a test SMTP service or Gmail.
"""

import os
import sys
import json
import hmac
import hashlib
import time
import requests

BASE_URL = "http://localhost:8080"
WEBHOOK_SECRET = os.environ.get("STRIPE_WEBHOOK_SECRET", "whsec_test_secret")

def generate_signature(payload: str, secret: str, timestamp: int) -> str:
    """Generate Stripe webhook signature"""
    signed_payload = f"{timestamp}.{payload}"
    signature = hmac.new(
        secret.encode('utf-8'),
        signed_payload.encode('utf-8'),
        hashlib.sha256
    ).hexdigest()
    return f"t={timestamp},v1={signature}"

def send_webhook(event_type: str, event_data: dict):
    """Send webhook event to server"""
    print(f"\n{'='*60}")
    print(f"TEST: {event_type}")
    print(f"{'='*60}")
    
    timestamp = int(time.time())
    payload = json.dumps({
        "id": f"evt_{int(time.time())}",
        "type": event_type,
        "data": {
            "object": event_data
        },
        "created": timestamp
    })
    
    signature = generate_signature(payload, WEBHOOK_SECRET, timestamp)
    
    headers = {
        "Content-Type": "application/json",
        "Stripe-Signature": signature
    }
    
    print(f"Payload: {json.dumps(json.loads(payload), indent=2)}")
    print(f"Sending webhook to {BASE_URL}/webhooks/stripe...")
    
    try:
        response = requests.post(
            f"{BASE_URL}/webhooks/stripe",
            data=payload,
            headers=headers,
            timeout=10
        )
        
        print(f"Status: {response.status_code}")
        
        if response.status_code == 200:
            print("✅ PASS: Webhook processed")
            try:
                print(f"Response: {json.dumps(response.json(), indent=2)}")
            except:
                print(f"Response: {response.text}")
            return True
        else:
            print(f"❌ FAIL: HTTP {response.status_code}")
            print(f"Response: {response.text}")
            return False
    except Exception as e:
        print(f"❌ ERROR: {str(e)}")
        return False

def test_payment_succeeded():
    """Test payment success email"""
    event_data = {
        "id": "in_test_payment_success",
        "customer": "cus_test_customer",
        "subscription": "sub_test",
        "amount_paid": 1000  # $10.00
    }
    return send_webhook("invoice.payment_succeeded", event_data)

def test_payment_failed():
    """Test payment failure email"""
    event_data = {
        "id": "in_test_payment_failed",
        "customer": "cus_test_customer",
        "subscription": "sub_test",
        "amount_due": 1000  # $10.00
    }
    return send_webhook("invoice.payment_failed", event_data)

def test_subscription_created():
    """Test subscription created email"""
    event_data = {
        "id": "sub_test_new",
        "customer": "cus_test_customer",
        "status": "active",
        "current_period_end": int(time.time()) + 30 * 24 * 3600
    }
    return send_webhook("customer.subscription.created", event_data)

def test_subscription_updated():
    """Test subscription updated email"""
    event_data = {
        "id": "sub_test",
        "customer": "cus_test_customer",
        "status": "active",
        "current_period_end": int(time.time()) + 30 * 24 * 3600
    }
    return send_webhook("customer.subscription.updated", event_data)

def test_subscription_deleted():
    """Test subscription canceled email"""
    event_data = {
        "id": "sub_test",
        "customer": "cus_test_customer",
        "status": "canceled",
        "current_period_end": int(time.time()) + 7 * 24 * 3600
    }
    return send_webhook("customer.subscription.deleted", event_data)

def test_payment_method_attached():
    """Test payment method attached email"""
    event_data = {
        "id": "pm_test_card",
        "type": "card",
        "card": {
            "last4": "4242",
            "brand": "Visa",
            "exp_month": 12,
            "exp_year": 2025
        }
    }
    return send_webhook("payment_method.attached", event_data)

def main():
    """Run all email notification tests"""
    print("\n" + "="*60)
    print("EMAIL NOTIFICATION TEST SUITE")
    print("="*60)
    
    # Check configuration
    print("\n📧 Configuration Check:")
    smtp_configured = all([
        os.environ.get("SMTP_HOST"),
        os.environ.get("SMTP_USERNAME"),
        os.environ.get("SMTP_PASSWORD")
    ])
    
    if smtp_configured:
        print(f"✅ SMTP configured:")
        print(f"   Host: {os.environ.get('SMTP_HOST')}")
        print(f"   Port: {os.environ.get('SMTP_PORT', '587')}")
        print(f"   From: {os.environ.get('FROM_EMAIL', 'noreply@nanolambda.com')}")
    else:
        print("⚠️  SMTP not configured - emails will be logged but not sent")
        print("   Set: SMTP_HOST, SMTP_USERNAME, SMTP_PASSWORD")
    
    print(f"\n🌐 Server: {BASE_URL}")
    print(f"🔑 Webhook Secret: {WEBHOOK_SECRET[:15]}...")
    
    # Check if server is running
    try:
        response = requests.get(f"{BASE_URL}/health", timeout=5)
        if response.status_code == 200:
            print("✅ Server is running")
        else:
            print(f"⚠️  Server responded with status {response.status_code}")
    except Exception as e:
        print(f"❌ ERROR: Cannot connect to server at {BASE_URL}")
        print(f"   Make sure the server is running: ./target/release/nanolambda-server")
        return 1
    
    # Run tests
    print("\n" + "="*60)
    print("RUNNING EMAIL TESTS")
    print("="*60)
    
    results = []
    
    # Test 1: Payment Succeeded
    results.append(("Payment Succeeded", test_payment_succeeded()))
    time.sleep(1)
    
    # Test 2: Payment Failed
    results.append(("Payment Failed", test_payment_failed()))
    time.sleep(1)
    
    # Test 3: Subscription Created
    results.append(("Subscription Created", test_subscription_created()))
    time.sleep(1)
    
    # Test 4: Subscription Updated
    results.append(("Subscription Updated", test_subscription_updated()))
    time.sleep(1)
    
    # Test 5: Subscription Deleted
    results.append(("Subscription Deleted", test_subscription_deleted()))
    time.sleep(1)
    
    # Test 6: Payment Method Attached
    results.append(("Payment Method Attached", test_payment_method_attached()))
    
    # Summary
    print("\n" + "="*60)
    print("TEST SUMMARY")
    print("="*60)
    
    passed = sum(1 for _, result in results if result)
    total = len(results)
    
    for name, result in results:
        status = "✅ PASS" if result else "❌ FAIL"
        print(f"{status}: {name}")
    
    print(f"\nPassed: {passed}/{total}")
    print(f"Failed: {total - passed}/{total}")
    
    if smtp_configured:
        print("\n📧 Check your email inbox for test emails!")
        print("   (Check spam folder if not in inbox)")
    else:
        print("\n⚠️  SMTP not configured - check server logs for email content")
    
    if passed == total:
        print("\n✅ ALL TESTS PASSED!")
        return 0
    else:
        print(f"\n❌ {total - passed} TESTS FAILED")
        return 1

if __name__ == "__main__":
    try:
        sys.exit(main())
    except KeyboardInterrupt:
        print("\n\n⚠️  Tests interrupted by user")
        sys.exit(130)
