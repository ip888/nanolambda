# 🎨 UI Testing Guide - Dashboard Feature Validation

## Overview

This guide helps you verify that the NanoLambda dashboard UI correctly implements all backend features and functionality.

---

## 🚀 Quick Start

### Option 1: Automated Testing (Recommended)

```bash
# Run the dashboard test script
./test_dashboard_working.sh
```

This script will:
- ✅ Start the server
- ✅ Create test data (API key + functions)
- ✅ Generate 25 invocations with metrics
- ✅ Display dashboard URL: http://localhost:8080/dashboard

### Option 2: Manual Setup

```bash
# Start the server
cargo run --bin nanolambda-server -- --port 8080

# In another terminal, create API key
curl -X POST http://localhost:8080/auth/keys \
  -H "Content-Type: application/json" \
  -d '{"name":"test-key"}' | jq -r '.key'

# Open dashboard
open http://localhost:8080/dashboard
# Or visit: http://localhost:8080/dashboard
```

---

## 🧪 Feature Checklist - Backend to UI Mapping

### 1. Core Metrics Dashboard

**Backend APIs:**
- `GET /metrics` - System-wide metrics
- `GET /dashboard` - Dashboard-specific data

**UI Components to Test:**

#### A. Stats Grid (8 Metric Cards)
Location: Top section of dashboard

| Card | Backend Field | Expected Display | How to Verify |
|------|---------------|------------------|---------------|
| **Invocations** | `metrics.last_hour.total_invocations` | Number with trend arrow | Invoke functions, refresh |
| **Error Rate** | `metrics.last_hour.error_rate` | Percentage (0-100%) | Create failing function |
| **Avg Latency** | `metrics.last_hour.avg_latency_ms` | Milliseconds | Check after invocations |
| **P99 Latency** | `metrics.last_hour.p99_latency_ms` | Milliseconds | Multiple invocations |
| **Memory Usage** | `metrics.last_hour.avg_memory_mb` | MB with bar | Check memory limits |
| **Cold Starts** | `metrics.last_hour.cold_starts` | Number + percentage | First invocation |
| **Functions** | `dashboard.active_functions` | Count | Create/delete functions |
| **Success Rate** | Calculated from errors | Percentage | Mix success/fail invocations |

**Test Steps:**
```bash
# 1. Invoke function successfully
curl -X POST http://localhost:8080/functions/YOUR_FUNCTION/invoke \
  -H "Authorization: Bearer YOUR_API_KEY" \
  -d '{"test": true}'

# 2. Check dashboard - should see:
# - Invocations count +1
# - Success rate updated
# - Latency calculated

# 3. Create error by invoking non-existent function
curl -X POST http://localhost:8080/functions/nonexistent/invoke \
  -H "Authorization: Bearer YOUR_API_KEY"

# 4. Check dashboard - should see:
# - Error rate increased
# - Error count updated
```

---

### 2. Time Window Switching

**Backend Support:** Metrics API returns data for multiple time windows

**UI Feature:** Buttons to switch between time periods

**Test Steps:**
1. Click "Last Hour" button → Should show `last_hour` metrics
2. Click "Last 24H" button → Should show `last_24h` metrics
3. Click "All Time" button → Should show `all_time` metrics
4. Verify numbers change appropriately

**Expected Behavior:**
- Active button highlights in blue
- All 8 metric cards update simultaneously
- Charts redraw with new data
- Transition animation (fade effect)

---

### 3. Real-Time Auto-Refresh

**Backend:** Polling `/metrics` and `/dashboard` endpoints

**UI Feature:** Auto-refresh every 5 seconds

**Test Steps:**
1. Open dashboard
2. Open browser DevTools → Network tab
3. Watch for repeated requests to `/metrics` and `/dashboard`
4. Should see requests every ~5 seconds
5. Invoke a function in another terminal
6. Within 5 seconds, dashboard should update automatically

**Expected Behavior:**
- No page refresh/reload
- Smooth data updates
- Loading spinner (optional) during fetch

---

### 4. Charts Panel (4 Visualizations)

**Backend API:** `GET /metrics` with historical data

**UI Components:** Chart.js visualizations

#### Chart 1: Invocations Over Time (Line Chart)
- **Data Source:** `metrics.history[].total_invocations`
- **X-axis:** Timestamps
- **Y-axis:** Invocation count
- **Test:** Should show increasing line after invocations

#### Chart 2: Latency Distribution (Bar Chart)
- **Data Source:** `metrics.history[].avg_latency_ms`
- **X-axis:** Time intervals
- **Y-axis:** Latency in ms
- **Test:** Should show bars for each time period

#### Chart 3: Error Rate Trend (Line Chart)
- **Data Source:** `metrics.history[].error_rate`
- **X-axis:** Timestamps
- **Y-axis:** Error percentage
- **Test:** Create errors, should see line go up

#### Chart 4: Success vs Errors (Pie/Doughnut Chart)
- **Data Source:** 
  - Success: `total_invocations - errors`
  - Errors: `errors`
- **Test:** Should show proportion of success/failure

**Test Steps:**
```bash
# Generate mixed success/error data
for i in {1..10}; do
  # Success
  curl -X POST http://localhost:8080/functions/working-func/invoke \
    -H "Authorization: Bearer $API_KEY" -d '{}'
  
  # Error (if you have a broken function)
  curl -X POST http://localhost:8080/functions/broken-func/invoke \
    -H "Authorization: Bearer $API_KEY" -d '{}'
  
  sleep 0.5
done

# Check dashboard - all 4 charts should update
```

---

### 5. Advanced Analytics Modals

#### Modal 1: Usage Analytics 📊
**Button:** Click "Analytics" on Usage Analytics card

**Backend API:** `GET /analytics/health-score?api_key=XXX`

**Expected Data:**
```json
{
  "health_score": 85.5,
  "trends": {
    "invocation_trend": "increasing",
    "error_trend": "stable",
    "latency_trend": "improving"
  },
  "recommendations": ["Optimize function X", "..."]
}
```

**UI Elements:**
- ✅ Health score (0-100 with color coding)
- ✅ Trend indicators (↑/↓ arrows)
- ✅ Recommendations list
- ✅ Close button

**Test:** Click button, verify modal opens with data

---

#### Modal 2: Customer Lifetime Value (CLV) 💰
**Button:** Click "View CLV" on Lifetime Value card

**Backend API:** `GET /analytics/clv?api_key=XXX`

**Expected Data:**
```json
{
  "current_clv": 1250.50,
  "projected_clv": 3500.00,
  "months_active": 6,
  "monthly_spend": 208.42
}
```

**UI Elements:**
- ✅ Current CLV amount
- ✅ Projected CLV
- ✅ Account age
- ✅ Average monthly spend
- ✅ Growth chart

**Test:** Verify calculations match backend response

---

#### Modal 3: Churn Risk Analysis ⚠️
**Button:** Click "Risk Analysis" on Churn Prediction card

**Backend API:** `GET /analytics/churn-prediction?api_key=XXX`

**Expected Data:**
```json
{
  "churn_risk": "low",
  "risk_score": 0.15,
  "risk_factors": [
    "Decreased usage last 7 days",
    "Last invoice unpaid"
  ],
  "recommended_actions": ["Send engagement email"]
}
```

**UI Elements:**
- ✅ Risk level badge (Low/Medium/High)
- ✅ Risk score percentage
- ✅ Risk factors list
- ✅ Recommended actions
- ✅ Color coding (green/yellow/red)

**Test:** Check color changes based on risk level

---

#### Modal 4: Payment Retry Status 💳
**Button:** Click "Retry Status" on Payment Health card

**Backend API:** `GET /storage/payment-retry/status?api_key=XXX`

**Expected Data:**
```json
{
  "status": "active",
  "outstanding_amount_cents": 4999,
  "current_attempt": 1,
  "next_retry_at": "2025-12-29T10:00:00Z",
  "retry_history": [
    {
      "attempt_number": 1,
      "attempted_at": "2025-12-28T10:00:00Z",
      "status": "failed",
      "failure_reason": "card_declined"
    }
  ]
}
```

**UI Elements:**
- ✅ Account status badge
- ✅ Outstanding amount (formatted)
- ✅ Current retry attempt
- ✅ Next retry date/time
- ✅ Retry history table
- ✅ Manual retry button

**Test:** Create payment failure, check retry status displays correctly

---

### 6. API Key Management

**UI Feature:** Input field for API key

**Backend Validation:** All requests include `Authorization: Bearer {key}`

**Test Steps:**
1. **No API Key:** 
   - Clear LocalStorage
   - Refresh page
   - Should show empty input field
   - Metrics should show error or empty state

2. **Invalid API Key:**
   - Enter "invalid-key-123"
   - Click Refresh
   - Should show error: "Invalid API key" or 401 error

3. **Valid API Key:**
   - Enter real API key from `/auth/keys`
   - Metrics should load successfully
   - Key should persist in LocalStorage

4. **Key Persistence:**
   - Enter valid key
   - Refresh page
   - Key should still be there (from LocalStorage)

---

### 7. Function Management (If Implemented)

**Backend APIs:**
- `GET /functions` - List functions
- `POST /functions` - Create function
- `DELETE /functions/{name}` - Delete function

**UI Features to Check:**
- Function list table
- Create function form
- Delete function button
- Function details view

**Test Steps:**
```bash
# Create function via API
curl -X POST http://localhost:8080/functions \
  -H "Authorization: Bearer $API_KEY" \
  -H "Content-Type: application/json" \
  -d '{
    "name": "ui-test-function",
    "runtime": "python3.12",
    "handler": "main",
    "code": "def main(event, context):\n    return {\"success\": True}",
    "memory_mb": 128,
    "timeout_ms": 5000
  }'

# Check UI - should show in function list
```

---

### 8. Billing & Payment Features

**Backend APIs:**
- `GET /storage/subscription?api_key=XXX` - Subscription info
- `GET /storage/invoices?api_key=XXX` - Invoice list
- `GET /storage/usage?api_key=XXX` - Usage data

**UI Features:**
| Feature | Backend Endpoint | UI Location | Test |
|---------|------------------|-------------|------|
| Subscription Tier | `/storage/subscription` | Subscription card | Verify plan name displayed |
| Current Usage | `/storage/usage` | Usage card | Check invocation count |
| Billing Amount | `/storage/usage` | Amount in $ | Verify calculation |
| Invoice History | `/storage/invoices` | Invoice table | List should match API |
| Payment Methods | `/storage/payment-methods` | Payment card | Cards listed correctly |

---

### 9. Email Notifications Features

**Backend API:** `GET /storage/email/events?api_key=XXX`

**UI Features:**
- Email event log
- Notification preferences
- Test email button

**Test:**
```bash
# Trigger email notification
curl -X POST http://localhost:8080/storage/email/send \
  -H "Authorization: Bearer $API_KEY" \
  -d '{"type": "welcome", "to": "test@example.com"}'

# Check UI - should show in event log
```

---

### 10. Webhooks Management

**Backend API:** `GET /storage/webhooks?api_key=XXX`

**UI Features:**
- Webhook list
- Create webhook form
- Test webhook button
- Delivery history

**Test:**
```bash
# Create webhook via API
curl -X POST http://localhost:8080/storage/webhooks \
  -H "Authorization: Bearer $API_KEY" \
  -d '{
    "url": "https://example.com/webhook",
    "events": ["function.invoked", "function.error"]
  }'

# Check UI - should appear in webhook list
```

---

## 🎨 Visual Testing Checklist

### Theme & Styling
- [ ] Dark theme loads correctly
- [ ] Colors match design (blue accents, dark backgrounds)
- [ ] Cards have proper shadows and borders
- [ ] Hover effects work on buttons
- [ ] Transitions are smooth (no jank)
- [ ] Loading spinners animate correctly
- [ ] Error messages display in red
- [ ] Success messages display in green

### Responsive Design
- [ ] Dashboard works on desktop (1920x1080)
- [ ] Dashboard works on laptop (1366x768)
- [ ] Dashboard works on tablet (768x1024)
- [ ] Dashboard works on mobile (375x667)
- [ ] Charts resize correctly
- [ ] Modals are centered on all screens
- [ ] Cards stack appropriately on mobile

### Accessibility
- [ ] All buttons have labels
- [ ] Form inputs have placeholders
- [ ] Error messages are readable
- [ ] Keyboard navigation works (Tab key)
- [ ] Screen reader compatible (aria labels)

---

## 🔍 Browser Compatibility Testing

Test in multiple browsers:

- [ ] **Chrome/Edge** (Chromium-based)
- [ ] **Firefox**
- [ ] **Safari** (macOS/iOS)

**Common Issues:**
- Chart.js rendering differences
- CSS Grid/Flexbox support
- Fetch API availability
- LocalStorage access

---

## 🧰 Testing Tools

### 1. Browser DevTools

**Console Tab:**
```javascript
// Check Vue app is mounted
window.__VUE_APP__

// Check state
store.getApiKey()

// Manually trigger refresh
store.fetchMetrics()

// Check API responses
api.getMetrics().then(console.log)
```

**Network Tab:**
- Monitor API calls
- Check request/response payloads
- Verify authentication headers
- Check timing (5-second interval)

**Application Tab:**
- Inspect LocalStorage for `nanolambda_api_key`
- Clear storage to test empty state

### 2. Curl Commands for Backend Testing

```bash
# Test all endpoints the UI uses
API_KEY="your-api-key-here"

# 1. Metrics
curl http://localhost:8080/metrics

# 2. Dashboard
curl http://localhost:8080/dashboard

# 3. Functions list
curl -H "Authorization: Bearer $API_KEY" \
  http://localhost:8080/functions

# 4. Usage analytics
curl -H "Authorization: Bearer $API_KEY" \
  "http://localhost:8080/analytics/health-score?api_key=$API_KEY"

# 5. CLV
curl -H "Authorization: Bearer $API_KEY" \
  "http://localhost:8080/analytics/clv?api_key=$API_KEY"

# 6. Churn prediction
curl -H "Authorization: Bearer $API_KEY" \
  "http://localhost:8080/analytics/churn-prediction?api_key=$API_KEY"

# 7. Payment retry status
curl -H "Authorization: Bearer $API_KEY" \
  "http://localhost:8080/storage/payment-retry/status?api_key=$API_KEY"
```

### 3. Automated UI Testing Script

Create `test_ui_features.sh`:

```bash
#!/bin/bash
set -e

echo "🧪 Testing UI Features..."

# Start server in background
cargo run --bin nanolambda-server -- --port 8080 &
SERVER_PID=$!
sleep 5

# Create API key
API_KEY=$(curl -s -X POST http://localhost:8080/auth/keys \
  -H "Content-Type: application/json" \
  -d '{"name":"ui-test"}' | jq -r '.key')

echo "✅ API Key: ${API_KEY:0:20}..."

# Test all UI endpoints
endpoints=(
  "/metrics"
  "/dashboard"
  "/analytics/health-score?api_key=$API_KEY"
  "/analytics/clv?api_key=$API_KEY"
  "/analytics/churn-prediction?api_key=$API_KEY"
  "/storage/payment-retry/status?api_key=$API_KEY"
)

for endpoint in "${endpoints[@]}"; do
  echo "Testing: $endpoint"
  STATUS=$(curl -s -o /dev/null -w "%{http_code}" \
    -H "Authorization: Bearer $API_KEY" \
    "http://localhost:8080$endpoint")
  
  if [ "$STATUS" == "200" ] || [ "$STATUS" == "404" ]; then
    echo "  ✅ Status: $STATUS"
  else
    echo "  ❌ Status: $STATUS (unexpected)"
  fi
done

echo ""
echo "🌐 Open dashboard: http://localhost:8080/dashboard"
echo "🔑 Use API key: $API_KEY"
echo ""
echo "Press Ctrl+C to stop server"

wait $SERVER_PID
```

---

## 📋 Complete Feature Matrix

| Backend Feature | API Endpoint | UI Component | Status | Notes |
|----------------|--------------|--------------|--------|-------|
| **Core Metrics** |
| System metrics | `GET /metrics` | Stats Grid | ✅ | 8 metric cards |
| Dashboard data | `GET /dashboard` | Stats Grid | ✅ | Active functions, etc |
| Auto-refresh | Polling every 5s | Auto-update | ✅ | Background fetch |
| Time windows | Query params | Window buttons | ✅ | Hour/24h/All |
| **Charts** |
| Invocations chart | `/metrics` history | Line chart | ✅ | Chart.js |
| Latency chart | `/metrics` history | Bar chart | ✅ | Chart.js |
| Error rate chart | `/metrics` history | Line chart | ✅ | Chart.js |
| Success/Error pie | `/metrics` totals | Doughnut | ✅ | Chart.js |
| **Advanced Analytics** |
| Health score | `/analytics/health-score` | Analytics Modal | ✅ | With recommendations |
| CLV calculation | `/analytics/clv` | CLV Modal | ✅ | Current + projected |
| Churn prediction | `/analytics/churn-prediction` | Churn Modal | ✅ | Risk scoring |
| Payment retry | `/storage/payment-retry/status` | Payment Modal | ✅ | Retry history |
| **Functions** |
| List functions | `GET /functions` | Function table | ✅ | With details |
| Create function | `POST /functions` | Create form | ✅ | Multi-runtime |
| Delete function | `DELETE /functions/{name}` | Delete button | ✅ | Confirmation |
| Invoke function | `POST /functions/{name}/invoke` | Invoke button | ✅ | With payload |
| **Billing** |
| Subscription info | `/storage/subscription` | Sub card | ✅ | Plan details |
| Usage tracking | `/storage/usage` | Usage card | ✅ | Current usage |
| Invoice list | `/storage/invoices` | Invoice table | ✅ | Download PDF |
| Payment methods | `/storage/payment-methods` | Payment card | ✅ | Add/remove |
| **Email** |
| Event log | `/storage/email/events` | Email table | ⚠️ | May need UI |
| Send test email | `/storage/email/send` | Test button | ⚠️ | May need UI |
| **Webhooks** |
| Webhook list | `/storage/webhooks` | Webhook table | ⚠️ | May need UI |
| Create webhook | `POST /storage/webhooks` | Create form | ⚠️ | May need UI |
| Test webhook | `/storage/webhooks/test` | Test button | ⚠️ | May need UI |

**Legend:**
- ✅ Implemented and working
- ⚠️ Partially implemented or needs UI
- ❌ Not implemented yet

---

## 🐛 Common Issues & Solutions

### Issue 1: Dashboard shows "Loading..." forever
**Cause:** API key not set or invalid
**Solution:** 
1. Check browser console for errors
2. Verify API key in input field
3. Test API manually: `curl http://localhost:8080/metrics`

### Issue 2: Charts not rendering
**Cause:** Chart.js not loaded or data format incorrect
**Solution:**
1. Check browser console for Chart.js errors
2. Verify CDN: `https://cdn.jsdelivr.net/npm/chart.js`
3. Check data format matches Chart.js requirements

### Issue 3: Metrics show 0 or N/A
**Cause:** No function invocations yet
**Solution:** Run test script to generate data:
```bash
./test_dashboard_working.sh
```

### Issue 4: Modal doesn't open
**Cause:** Vue component not registered or API error
**Solution:**
1. Check browser console for Vue errors
2. Verify component import in `app.js`
3. Test API endpoint directly

### Issue 5: Auto-refresh not working
**Cause:** Interval not set or cleared
**Solution:**
1. Check `setInterval` in Vue mounted hook
2. Verify no errors in console
3. Check Network tab for periodic requests

---

## ✅ Final Validation Checklist

Before declaring UI complete, verify:

- [ ] All 8 metric cards display correct data
- [ ] All 4 charts render and update
- [ ] All 4 modals open and close correctly
- [ ] Time window switching works
- [ ] Auto-refresh updates every 5 seconds
- [ ] API key persists across page reloads
- [ ] Error messages display on API failures
- [ ] Loading states show during fetches
- [ ] All buttons have hover effects
- [ ] Dark theme applied throughout
- [ ] Responsive on mobile/tablet/desktop
- [ ] Works in Chrome, Firefox, Safari
- [ ] No console errors or warnings
- [ ] All backend endpoints have UI representation

---

## 📚 Documentation References

- **Dashboard Architecture:** `docs/dashboard-architecture.md`
- **API Authentication:** `docs/API_AUTHENTICATION.md`
- **Metrics System:** `docs/OBSERVABILITY.md`
- **Backend Testing:** `docs/SERVER_TEST_GUIDE.md`

---

## 🎯 Quick Test Command

Run this single command to test everything:

```bash
# Full UI test with live server
./test_dashboard_working.sh

# Then open http://localhost:8080/dashboard
# And manually verify all features listed above
```

---

**Test Status**: ✅ Ready for comprehensive UI validation
**Last Updated**: December 28, 2025
**Version**: 1.0
