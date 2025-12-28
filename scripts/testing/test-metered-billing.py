#!/usr/bin/env python3
"""
Test script for metered billing functionality.
Tests usage reporting and overage calculation.
"""

import requests
import json
import sys
from datetime import datetime

BASE_URL = "http://localhost:8080"
API_KEY = "test-api-key-123"  # Replace with actual API key

def test_report_usage(quantity: int, timestamp: int = None):
    """Test reporting usage to Stripe"""
    print(f"\n{'='*60}")
    print(f"TEST: Report Usage - {quantity} invocations")
    print(f"{'='*60}")
    
    url = f"{BASE_URL}/payment/usage"
    headers = {
        "Authorization": f"Bearer {API_KEY}",
        "Content-Type": "application/json"
    }
    
    payload = {"quantity": quantity}
    if timestamp:
        payload["timestamp"] = timestamp
    
    print(f"Request: POST {url}")
    print(f"Payload: {json.dumps(payload, indent=2)}")
    
    try:
        response = requests.post(url, json=payload, headers=headers)
        print(f"Status: {response.status_code}")
        print(f"Response: {json.dumps(response.json(), indent=2)}")
        
        if response.status_code == 200:
            data = response.json()
            if data.get("success"):
                print("✅ PASS: Usage reported successfully")
                return True
            else:
                print(f"❌ FAIL: {data.get('message', 'Unknown error')}")
                return False
        else:
            print(f"❌ FAIL: HTTP {response.status_code}")
            return False
    except Exception as e:
        print(f"❌ ERROR: {str(e)}")
        return False

def test_calculate_overage():
    """Test calculating overage costs"""
    print(f"\n{'='*60}")
    print(f"TEST: Calculate Overage")
    print(f"{'='*60}")
    
    url = f"{BASE_URL}/payment/overage"
    headers = {
        "Authorization": f"Bearer {API_KEY}",
        "Content-Type": "application/json"
    }
    
    print(f"Request: GET {url}")
    
    try:
        response = requests.get(url, headers=headers)
        print(f"Status: {response.status_code}")
        print(f"Response: {json.dumps(response.json(), indent=2)}")
        
        if response.status_code == 200:
            data = response.json()
            if data.get("success"):
                print("\n📊 Overage Analysis:")
                print(f"  Current Usage: {data.get('current_usage', 0):,} invocations")
                print(f"  Tier Limit: {data.get('tier_limit', 'N/A')}")
                print(f"  Overage: {data.get('overage_invocations', 0):,} invocations")
                print(f"  Overage Cost: ${data.get('overage_cost', 0):.4f} {data.get('currency', 'USD').upper()}")
                print("✅ PASS: Overage calculated successfully")
                return True, data
            else:
                print(f"❌ FAIL: {data.get('message', 'Unknown error')}")
                return False, None
        else:
            print(f"❌ FAIL: HTTP {response.status_code}")
            return False, None
    except Exception as e:
        print(f"❌ ERROR: {str(e)}")
        return False, None

def test_scenario(name: str, description: str, usage_quantity: int):
    """Test a specific usage scenario"""
    print(f"\n{'#'*60}")
    print(f"SCENARIO: {name}")
    print(f"Description: {description}")
    print(f"{'#'*60}")
    
    # Report usage
    if test_report_usage(usage_quantity):
        # Calculate overage
        success, data = test_calculate_overage()
        return success
    return False

def main():
    """Run all metered billing tests"""
    print("\n" + "="*60)
    print("METERED BILLING TEST SUITE")
    print("="*60)
    
    results = []
    
    # Test 1: Low usage (no overage)
    results.append(test_scenario(
        name="Low Usage (No Overage)",
        description="User within tier limits (100k invocations)",
        usage_quantity=100000
    ))
    
    # Test 2: Medium usage (50% of limit)
    results.append(test_scenario(
        name="Medium Usage (50% of Limit)",
        description="User at 50% of Starter tier limit (500k invocations)",
        usage_quantity=500000
    ))
    
    # Test 3: Near limit (90%)
    results.append(test_scenario(
        name="Near Limit (90%)",
        description="User at 90% of Starter tier limit (900k invocations)",
        usage_quantity=900000
    ))
    
    # Test 4: At limit (100%)
    results.append(test_scenario(
        name="At Limit (100%)",
        description="User exactly at Starter tier limit (1M invocations)",
        usage_quantity=1000000
    ))
    
    # Test 5: Over limit (110%)
    results.append(test_scenario(
        name="Over Limit (110%)",
        description="User 10% over Starter tier limit (1.1M invocations)",
        usage_quantity=1100000
    ))
    
    # Test 6: Significant overage (150%)
    results.append(test_scenario(
        name="Significant Overage (150%)",
        description="User 50% over Starter tier limit (1.5M invocations)",
        usage_quantity=1500000
    ))
    
    # Test 7: Calculate current overage without reporting
    print(f"\n{'#'*60}")
    print(f"SCENARIO: Current Overage Check")
    print(f"Description: Check current overage without reporting new usage")
    print(f"{'#'*60}")
    success, data = test_calculate_overage()
    results.append(success)
    
    # Summary
    print("\n" + "="*60)
    print("TEST SUMMARY")
    print("="*60)
    passed = sum(1 for r in results if r)
    total = len(results)
    print(f"Passed: {passed}/{total}")
    print(f"Failed: {total - passed}/{total}")
    
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
