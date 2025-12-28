# Subscription Upgrade Prompts - NanoLambda

## Overview

NanoLambda's intelligent upgrade system automatically analyzes your usage patterns and provides personalized recommendations to help you choose the right tier. The system considers multiple factors to determine when and how to suggest upgrades, ensuring you get optimal value without over-provisioning.

## Key Features

- **Smart Recommendations**: AI-powered analysis of usage patterns
- **Urgency Scoring**: Prioritizes recommendations based on actual need (0-100 scale)
- **In-App Upgrade Flow**: Complete upgrade process without leaving the dashboard
- **Cost Comparison**: Side-by-side preview of current vs. recommended tier
- **Proactive Alerts**: Recommendations appear before you hit limits
- **Dismissible Banners**: Non-intrusive UI that respects user preference

## How Recommendations Work

### Trigger Conditions

The system recommends upgrades when you meet ANY of these criteria:

1. **Usage Threshold**: Using ≥50% of monthly invocation limit
2. **High Usage**: Using ≥70% triggers moderate recommendation
3. **Critical Usage**: Using ≥85% triggers urgent recommendation  
4. **Capacity Benefits**: Significant feature improvements available in next tier

### Urgency Levels

The urgency score determines recommendation priority:

| Urgency | Score Range | Usage % | Banner Color | Label |
|---------|-------------|---------|--------------|-------|
| **Critical** | 80-100 | ≥95% | Red (#ef4444) | URGENT |
| **High** | 60-79 | 70-94% | Orange (#f97316) | RECOMMENDED |
| **Moderate** | 40-59 | 50-69% | Blue (#3b82f6) | SUGGESTED |
| **Low** | 20-39 | <50% | - | (No prompt) |

### Recommendation Algorithm

```
IF usage_percent >= 95%:
    urgency = 100 (CRITICAL)
ELSE IF usage_percent >= 85%:
    urgency = 80 (HIGH)
ELSE IF usage_percent >= 70%:
    urgency = 60 (HIGH)
ELSE IF usage_percent >= 50%:
    urgency = 40 (MODERATE)
ELSE:
    No recommendation

BENEFITS = Calculate(current_tier, next_tier)
REASONS = Build_Reasons_List(usage, limits, benefits)

RETURN Recommendation(urgency, reasons, benefits)
```

## API Endpoints

### Get Upgrade Recommendation

Retrieve a personalized upgrade recommendation for the authenticated user:

```bash
GET /tier/recommendation
Authorization: Bearer YOUR_API_KEY

# Response (with recommendation)
{
  "success": true,
  "has_recommendation": true,
  "recommendation": {
    "current_tier": "starter",
    "recommended_tier": "pro",
    "urgency": 80,
    "usage_percent": 87,
    "current_usage": 870000,
    "current_limit": 1000000,
    "reasons": [
      "You've used 87% of your monthly invocation limit",
      "You're at risk of hitting your monthly limit",
      "Get 10x more invocations",
      "Unlock 1024MB more memory",
      "Access advanced monitoring features"
    ],
    "estimated_savings": null
  }
}

# Response (no recommendation)
{
  "success": true,
  "has_recommendation": false,
  "message": "No upgrade recommended at this time"
}
```

**When to Use**: 
- Display upgrade suggestions in your dashboard
- Check after significant usage spikes
- Integrate into automated monitoring systems

**Recommendation Criteria**:
- Only suggests upgrades for Starter → Pro or Pro → Enterprise
- Returns `null` if usage is below 50%
- No recommendations for Enterprise tier users

### Get Upgrade Preview

Preview detailed comparison between current and target tier:

```bash
GET /tier/preview?tier=pro
Authorization: Bearer YOUR_API_KEY

# Response
{
  "success": true,
  "current": {
    "tier": "starter",
    "name": "Starter",
    "invocations_per_month": 1000000,
    "memory_mb": 512,
    "timeout_ms": 30000,
    "concurrent_executions": 10
  },
  "target": {
    "tier": "pro",
    "name": "Pro",
    "invocations_per_month": 10000000,
    "memory_mb": 1536,
    "timeout_ms": 60000,
    "concurrent_executions": 50
  },
  "improvements": {
    "invocation_increase": 9000000,
    "memory_increase_mb": 1024,
    "timeout_increase_ms": 30000,
    "concurrent_increase": 40,
    "new_features": [
      "Custom domains",
      "Advanced monitoring",
      "Priority execution",
      "Enhanced support"
    ]
  },
  "current_usage": {
    "monthly_invocations": 870000,
    "usage_percent": 87
  }
}
```

**Query Parameters**:
- `tier` (required): Target tier to preview - `"starter"`, `"pro"`, or `"enterprise"`

**Use Cases**:
- Display upgrade benefits before user commits
- Build custom comparison UIs
- A/B test different messaging approaches

### Perform Upgrade

Execute the tier upgrade:

```bash
PUT /tier/upgrade
Authorization: Bearer YOUR_API_KEY
Content-Type: application/json

{
  "tier": "pro"
}

# Response
{
  "success": true,
  "message": "Successfully upgraded to Pro tier",
  "tier": "pro",
  "name": "Pro",
  "assigned_at": 1702483200
}
```

**Prerequisites**:
- Active payment method on file
- Valid Stripe subscription

**Post-Upgrade**:
- Tier limits update immediately
- Billing reflects new pricing at next cycle
- Features unlock instantly

## Dashboard Integration

### Upgrade Banner

The dashboard automatically displays intelligent upgrade banners based on usage:

#### Banner Appearance

**Urgent (Red)**:
```
┌─────────────────────────────────────────────────────────────┐
│ URGENT UPGRADE                                              │
│                                                             │
│ Upgrade to PRO                                              │
│                                                             │
│ You're at 95% of your monthly limit (950,000 / 1,000,000   │
│ invocations)                                                │
│                                                             │
│ ✓ You've used 95% of your monthly invocation limit         │
│ ✓ You're at risk of hitting your monthly limit             │
│ ✓ Get 10x more invocations                                 │
│                                                             │
│                                    [View Upgrade →]         │
│                                    [Dismiss]                │
└─────────────────────────────────────────────────────────────┘
```

**Recommended (Orange)**:
```
┌─────────────────────────────────────────────────────────────┐
│ RECOMMENDED UPGRADE                                         │
│                                                             │
│ Upgrade to PRO                                              │
│                                                             │
│ You're at 75% of your monthly limit (750,000 / 1,000,000   │
│ invocations)                                                │
│                                                             │
│ ✓ You've used 75% of your monthly invocation limit         │
│ ✓ Get 10x more invocations                                 │
│ ✓ Unlock 1024MB more memory                                │
│                                                             │
│                                    [View Upgrade →]         │
│                                    [Dismiss]                │
└─────────────────────────────────────────────────────────────┘
```

### Upgrade Modal

Clicking "View Upgrade" opens a comprehensive comparison modal:

```
┌─────────────────────────────────────────────────────────────┐
│ Upgrade to Pro                                          [×] │
│ Currently on Starter plan (75% used)                        │
│                                                             │
│ What You'll Get                                             │
│ ┌─────────────────────────────────────────────────────────┐ │
│ │ Invocations/Month    │ Memory Limit                     │ │
│ │ 10,000,000           │ 1536MB                           │ │
│ │ +9.0M                │ +1024MB                          │ │
│ │                      │                                  │ │
│ │ Timeout              │ Concurrent Executions            │ │
│ │ 60s                  │ 50                               │ │
│ │ +30s                 │ +40                              │ │
│ │                                                         │ │
│ │ New Features Unlocked                                  │ │
│ │ ✨ Custom domains                                       │ │
│ │ ✨ Advanced monitoring                                  │ │
│ │ ✨ Priority execution                                   │ │
│ │ ✨ Enhanced support                                     │ │
│ └─────────────────────────────────────────────────────────┘ │
│                                                             │
│ [        Upgrade Now        ] [  Cancel  ]                  │
│                                                             │
│ You'll be redirected to complete payment setup              │
└─────────────────────────────────────────────────────────────┘
```

### In-App Upgrade Flow

1. **Recommendation Appears**: Banner shows based on usage analysis
2. **View Details**: Click "View Upgrade" to see comparison
3. **Review Benefits**: Modal displays side-by-side comparison
4. **Confirm Upgrade**: Click "Upgrade Now"
5. **Payment Check**: System verifies payment method exists
6. **Execute Upgrade**: Tier changes immediately
7. **Confirmation**: Success message displays
8. **Dashboard Refresh**: Automatic refresh shows new limits

### Payment Method Check

If no payment method is configured:
```javascript
// Automatic redirect to Customer Portal
alert('Please set up a payment method first');
openCustomerPortal(); // Opens Stripe portal in new tab
```

## Integration Examples

### JavaScript - Check Recommendation

```javascript
async function checkUpgradeRecommendation() {
  const response = await fetch('/tier/recommendation', {
    headers: {
      'Authorization': `Bearer ${apiKey}`
    }
  });
  
  const data = await response.json();
  
  if (data.has_recommendation) {
    const rec = data.recommendation;
    console.log(`💡 Upgrade to ${rec.recommended_tier} recommended!`);
    console.log(`Urgency: ${rec.urgency}/100`);
    console.log(`Current usage: ${rec.usage_percent}%`);
    console.log(`Reasons: ${rec.reasons.join(', ')}`);
    
    // Show upgrade UI
    displayUpgradePrompt(rec);
  } else {
    console.log('✅ No upgrade needed at this time');
  }
}
```

### Python - Automated Upgrade Checks

```python
import requests
from datetime import datetime

class UpgradeMonitor:
    def __init__(self, api_key):
        self.api_key = api_key
        self.base_url = 'https://api.nanolambda.com'
    
    def check_recommendation(self):
        """Check if upgrade is recommended"""
        response = requests.get(
            f'{self.base_url}/tier/recommendation',
            headers={'Authorization': f'Bearer {self.api_key}'}
        )
        
        data = response.json()
        
        if data['has_recommendation']:
            rec = data['recommendation']
            
            # Log recommendation
            print(f"[{datetime.now()}] Upgrade Recommendation:")
            print(f"  Current: {rec['current_tier']}")
            print(f"  Recommended: {rec['recommended_tier']}")
            print(f"  Urgency: {rec['urgency']}/100")
            print(f"  Usage: {rec['usage_percent']}%")
            
            # Send alert if urgent
            if rec['urgency'] >= 80:
                self.send_slack_alert(rec)
            
            return rec
        
        return None
    
    def preview_upgrade(self, target_tier):
        """Get detailed upgrade preview"""
        response = requests.get(
            f'{self.base_url}/tier/preview',
            params={'tier': target_tier},
            headers={'Authorization': f'Bearer {self.api_key}'}
        )
        
        preview = response.json()
        
        print(f"\n📊 Upgrade Preview: {target_tier.upper()}")
        print(f"Current Tier: {preview['current']['name']}")
        print(f"\nImprovements:")
        for key, value in preview['improvements'].items():
            if isinstance(value, list):
                print(f"  {key}: {', '.join(value)}")
            else:
                print(f"  {key}: +{value:,}")
        
        return preview
    
    def send_slack_alert(self, recommendation):
        """Send Slack notification for urgent upgrades"""
        # Implementation here
        pass

# Usage
monitor = UpgradeMonitor('your_api_key')
recommendation = monitor.check_recommendation()

if recommendation and recommendation['urgency'] >= 60:
    preview = monitor.preview_upgrade(recommendation['recommended_tier'])
```

### React Component

```jsx
import React, { useState, useEffect } from 'react';

function UpgradeBanner({ apiKey }) {
  const [recommendation, setRecommendation] = useState(null);
  const [showModal, setShowModal] = useState(false);
  const [preview, setPreview] = useState(null);
  
  useEffect(() => {
    checkRecommendation();
  }, [apiKey]);
  
  async function checkRecommendation() {
    try {
      const response = await fetch('/tier/recommendation', {
        headers: { 'Authorization': `Bearer ${apiKey}` }
      });
      
      const data = await response.json();
      
      if (data.has_recommendation) {
        setRecommendation(data.recommendation);
      }
    } catch (error) {
      console.error('Failed to fetch recommendation:', error);
    }
  }
  
  async function loadPreview() {
    if (!recommendation) return;
    
    const response = await fetch(
      `/tier/preview?tier=${recommendation.recommended_tier}`,
      { headers: { 'Authorization': `Bearer ${apiKey}` } }
    );
    
    const data = await response.json();
    setPreview(data);
    setShowModal(true);
  }
  
  if (!recommendation) return null;
  
  const urgencyColor = recommendation.urgency >= 80 
    ? '#ef4444' 
    : recommendation.urgency >= 60 
    ? '#f97316' 
    : '#3b82f6';
  
  return (
    <>
      <div style={{
        background: `linear-gradient(135deg, ${urgencyColor}, ${urgencyColor}dd)`,
        color: 'white',
        padding: '20px',
        borderRadius: '12px',
        marginBottom: '20px'
      }}>
        <h3>Upgrade to {recommendation.recommended_tier.toUpperCase()}</h3>
        <p>
          You're at {recommendation.usage_percent}% of your monthly limit
        </p>
        <ul>
          {recommendation.reasons.slice(0, 3).map((reason, i) => (
            <li key={i}>{reason}</li>
          ))}
        </ul>
        <button onClick={loadPreview}>View Upgrade Details</button>
      </div>
      
      {showModal && preview && (
        <UpgradeModal 
          preview={preview}
          onClose={() => setShowModal(false)}
          onUpgrade={() => performUpgrade(recommendation.recommended_tier)}
        />
      )}
    </>
  );
}
```

## Best Practices

### For Users

1. **Act on Urgent Recommendations**: Don't wait until 100% usage
2. **Review Preview**: Always check the upgrade preview before committing
3. **Plan Ahead**: Upgrade during low-traffic periods when possible
4. **Monitor Patterns**: Track weekly usage to anticipate needs
5. **Use Billing Portal**: Manage all billing through the Customer Portal

### For Administrators

1. **Monitor Recommendation Rates**: Track how many users see prompts vs. upgrade
2. **A/B Test Urgency Thresholds**: Optimize trigger points for your user base
3. **Customize Messaging**: Tailor reasons based on your pricing model
4. **Set Up Webhooks**: Get notified when users upgrade
5. **Analyze Conversion**: Track prompt → upgrade conversion rates

### Recommendation Tuning

Customize thresholds in `crates/storage/src/tier.rs`:

```rust
// Adjust urgency calculation
let urgency = if usage_percent >= 95.0 {
    100  // CRITICAL - Change to 98.0 for earlier alerts
} else if usage_percent >= 85.0 {
    80   // HIGH - Change to 80.0 for earlier alerts
} else if usage_percent >= 70.0 {
    60   // MODERATE - Change to 65.0 for different threshold
} else if usage_percent >= 50.0 {
    40   // LOW
} else {
    20
};

// Adjust minimum threshold for showing recommendations
if usage_percent < 50.0 && urgency < 60 {
    return Ok(None);  // Change 50.0 to 40.0 for earlier prompts
}
```

## Troubleshooting

### No Recommendations Showing

**Problem**: Dashboard doesn't display upgrade banner despite high usage

**Solutions**:
1. **Check Usage Level**:
   ```bash
   curl -X GET http://localhost:8080/tier/current \
     -H "Authorization: Bearer YOUR_KEY"
   ```
   Verify `usage_percent` is >= 50%

2. **Verify API Response**:
   ```bash
   curl -X GET http://localhost:8080/tier/recommendation \
     -H "Authorization: Bearer YOUR_KEY"
   ```
   Check `has_recommendation` field

3. **Already at Highest Tier**: Enterprise users don't get recommendations

### Upgrade Failed

**Problem**: "Upgrade failed" error when clicking "Upgrade Now"

**Solutions**:
1. **Check Payment Method**:
   ```bash
   curl -X GET http://localhost:8080/payment/customer \
     -H "Authorization: Bearer YOUR_KEY"
   ```
   Ensure `payment_method_id` exists

2. **Set Up Payment**: Click "Manage Billing" → Add payment method

3. **Verify Subscription**:
   - Check Stripe Dashboard for active subscription
   - Ensure subscription is not cancelled or past_due

### Wrong Urgency Level

**Problem**: Recommendation shows wrong urgency (too aggressive or too passive)

**Solutions**:
1. **Review Current Usage**:
   ```sql
   SELECT monthly_invocations, tier FROM user_tiers WHERE api_key = 'your_key';
   ```

2. **Check Tier Limits**:
   ```sql
   -- Starter: 1M, Pro: 10M, Enterprise: Unlimited
   ```

3. **Adjust Thresholds**: See "Recommendation Tuning" section above

### Modal Not Opening

**Problem**: Clicking "View Upgrade" doesn't show modal

**Solutions**:
1. **Check Console**: Look for JavaScript errors
2. **Verify API**: Test `/tier/preview` endpoint manually
3. **Clear Cache**: Hard refresh (Ctrl+Shift+R / Cmd+Shift+R)
4. **Check Z-Index**: Ensure modal CSS `z-index: 10000` isn't overridden

## Roadmap

Future enhancements:

- [ ] **ML-Based Forecasting**: Predict when user will hit limits
- [ ] **Usage Projections**: "You'll reach 100% in 7 days"
- [ ] **Seasonal Patterns**: Detect weekly/monthly usage trends
- [ ] **Cost Optimization**: Suggest downgrades during low usage
- [ ] **A/B Testing Framework**: Built-in experimentation for messaging
- [ ] **Custom Triggers**: Admin-defined recommendation rules
- [ ] **Multi-Dimensional Scoring**: Consider memory, timeout, errors
- [ ] **Personalized Reasons**: ML-generated recommendations
- [ ] **Smart Timing**: Recommend upgrades at optimal times
- [ ] **Downgrade Protection**: Warn before dropping tiers

## Related Documentation

- [Tiered Pricing](./tiered-pricing.md) - Understanding tier limits and pricing
- [Usage Alerts](./usage-alerts.md) - Automatic threshold notifications
- [Customer Portal](./customer-portal.md) - Self-service billing management
- [Metered Billing](./metered-billing.md) - Usage tracking and overage charges

## Support

Questions about upgrade recommendations?

- **Documentation**: https://docs.nanolambda.com/upgrade-prompts
- **Email**: support@nanolambda.com
- **Community**: https://community.nanolambda.com/upgrade-help
- **Sales**: sales@nanolambda.com (Enterprise inquiries)
