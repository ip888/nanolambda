# Usage Alerts - NanoLambda

## Overview

NanoLambda's Usage Alerts system automatically monitors your monthly invocation usage and sends email notifications when you approach or exceed your tier limits. This helps you:

- **Avoid Surprises**: Get notified before hitting your monthly limit
- **Plan Ahead**: Upgrade your tier proactively to avoid service interruption
- **Monitor Costs**: Track when you're approaching overage charges
- **Stay Informed**: Receive automatic alerts at 80%, 90%, and 100% thresholds

## Alert Thresholds

The system monitors three critical usage thresholds:

### 🟡 80% Warning (Early Notice)
- **When**: You've used 80% of your monthly invocation limit
- **Purpose**: Early warning to monitor usage or consider upgrading
- **Action**: Review your usage patterns and plan accordingly
- **Email**: Includes current usage, limit, and upgrade options

### 🟠 90% Warning (Urgent Notice)
- **When**: You've used 90% of your monthly invocation limit
- **Purpose**: Urgent notification that you're approaching your limit
- **Action**: Consider upgrading immediately to avoid service issues
- **Email**: Highlights urgency and provides upgrade path

### 🔴 100% Critical Alert (Limit Reached)
- **When**: You've reached or exceeded your monthly limit
- **Purpose**: Critical notification that your limit has been reached
- **Action**: Upgrade immediately or expect throttling/overage charges
- **Email**: Clear indication that service may be affected

## Email Notifications

### Email Template

When an alert is triggered, you'll receive an email with:

**Subject**: `🚨 Usage Alert: 90% of Limit Reached - NanoLambda`

**Content**:
- Usage percentage (80%, 90%, or 100%)
- Current invocations vs. monthly limit
- Visual progress bar showing usage
- Severity-based messaging:
  - **80%**: "Monitor your usage to avoid unexpected charges"
  - **90%**: "You're approaching your limit. Consider upgrading"
  - **100%**: "Limit exceeded. New invocations may be throttled"
- Link to view usage details
- Upgrade recommendations

### Example Email (90% Alert)

```
⚠️ WARNING Usage Alert

Your NanoLambda account has reached 90% of your monthly invocation limit.

Current Usage: 900,000 / 1,000,000 invocations

WARNING: You're approaching your monthly limit. Consider upgrading to 
avoid service interruption.

[View Usage Details Button]

Need more capacity? Consider upgrading to a higher tier.
```

## API Endpoints

### Check and Send Alerts

Manually trigger usage alert checking (also runs automatically):

```bash
POST /usage/check-alerts
Authorization: Bearer YOUR_API_KEY

# Response
{
  "success": true,
  "alerts_sent": 1,
  "alerts": [
    {
      "type": "warning90",
      "threshold_percent": 90,
      "current_usage": 900000,
      "usage_limit": 1000000,
      "sent_at": 1702483200
    }
  ]
}
```

**Alert Types**:
- `warning80`: 80% threshold reached
- `warning90`: 90% threshold reached
- `critical100`: 100% threshold reached

### Get Alert History

Retrieve your alert history (last 50 alerts):

```bash
GET /usage/alerts
Authorization: Bearer YOUR_API_KEY

# Response
{
  "success": true,
  "alerts": [
    {
      "id": 123,
      "type": "warning90",
      "threshold_percent": 90,
      "current_usage": 900000,
      "usage_limit": 1000000,
      "sent_at": 1702483200,
      "period_start": 1701878400,
      "period_end": 1704470400
    },
    {
      "id": 122,
      "type": "warning80",
      "threshold_percent": 80,
      "current_usage": 800000,
      "usage_limit": 1000000,
      "sent_at": 1702397600,
      "period_start": 1701878400,
      "period_end": 1704470400
    }
  ]
}
```

## Automatic Alert Checking

Alerts are automatically checked in several scenarios:

1. **After Each Invocation**: The system checks your usage percentage after recording each function invocation
2. **Periodic Background Jobs**: Hourly checks ensure alerts aren't missed
3. **API Calls**: Manual triggers via `/usage/check-alerts` endpoint
4. **Dashboard Loads**: When you view your dashboard, alerts are checked

### Alert Deduplication

The system prevents duplicate alerts:
- **One alert per threshold per billing period**: You'll only receive each alert (80%, 90%, 100%) once per monthly billing cycle
- **Database Tracking**: All sent alerts are recorded in the `usage_alerts` table
- **Period-Based**: Alerts reset at the start of each new billing month

## Dashboard Integration

The usage dashboard displays real-time alert indicators:

### Visual Indicators

**Tier Card Alert Badges**:
- 🟡 **80% USED**: Yellow badge, moderate concern
- 🟠 **90% USED**: Orange badge, high concern  
- 🔴 **LIMIT REACHED**: Red badge, critical concern

**Progress Bar Colors**:
- Green (0-79%): Normal usage
- Yellow (80-89%): Approaching limit
- Orange (90-99%): Near limit
- Red (100%+): Limit exceeded

### Example Dashboard Display

```
┌─────────────────────────────────────┐
│ 🎯 Current Tier                     │
│                                     │
│ STARTER                             │
│                                     │
│ 900,000 / 1,000,000 invocations    │
│ ⚠️ 90% USED                         │
│                                     │
│ ▓▓▓▓▓▓▓▓▓░ 90%                     │
│                                     │
│ Memory: 512MB | Timeout: 30s       │
│ [⚙️ Manage Billing]                 │
└─────────────────────────────────────┘
```

## Database Schema

Usage alerts are stored in the `usage_alerts` table:

```sql
CREATE TABLE usage_alerts (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    api_key TEXT NOT NULL,
    alert_type TEXT NOT NULL,           -- 'warning80', 'warning90', 'critical100'
    threshold_percent INTEGER NOT NULL,  -- 80, 90, or 100
    current_usage INTEGER NOT NULL,      -- Invocations at time of alert
    usage_limit INTEGER NOT NULL,        -- Monthly limit for tier
    sent_at INTEGER NOT NULL,            -- Unix timestamp when sent
    period_start INTEGER NOT NULL,       -- Billing period start
    period_end INTEGER NOT NULL          -- Billing period end
);

CREATE INDEX idx_usage_alerts_api_key_type_period 
    ON usage_alerts(api_key, alert_type, period_start);
```

## Best Practices

### For Users

1. **Set Up Email**: Ensure your Stripe customer account has a valid email address
2. **Monitor Regularly**: Check your dashboard frequently to track usage
3. **Act on Warnings**: Don't wait until 100% - upgrade at 80-90%
4. **Plan Capacity**: Review your usage trends monthly
5. **Use Alerts API**: Integrate alerts into your monitoring systems

### For Administrators

1. **Configure SMTP**: Set up reliable email delivery (see Email Notifications docs)
2. **Test Alerts**: Use test scenarios to verify email delivery
3. **Monitor System**: Track alert delivery rates and failures
4. **Adjust Thresholds**: Consider customer feedback on timing
5. **Backup Notifications**: Consider SMS or webhook integrations

## Configuration

### Environment Variables

Email alerts require SMTP configuration (see `docs/email-notifications.md`):

```bash
# Required for email alerts
SMTP_HOST=smtp.sendgrid.net
SMTP_PORT=587
SMTP_USERNAME=apikey
SMTP_PASSWORD=your_sendgrid_api_key
SMTP_FROM_EMAIL=noreply@nanolambda.com
SMTP_FROM_NAME="NanoLambda Platform"
```

### Customizing Thresholds

Currently, thresholds are fixed at 80%, 90%, and 100%. To modify:

1. Edit `crates/storage/src/payment.rs`
2. Update the `check_usage_alerts()` method
3. Modify threshold percentages in the if statements
4. Update email templates in `send_email_notification()`

```rust
// Example: Add 75% threshold
if percent_used >= 75 && !self.was_alert_sent(...) {
    let alert = self.send_usage_alert(
        api_key,
        UsageAlertType::Warning75,
        75,
        ...
    ).await?;
}
```

## Integration Examples

### Node.js - Check Alerts in App

```javascript
const axios = require('axios');

async function checkMyUsageAlerts() {
  try {
    const response = await axios.post(
      'https://api.nanolambda.com/usage/check-alerts',
      {},
      {
        headers: {
          'Authorization': `Bearer ${process.env.NANOLAMBDA_API_KEY}`
        }
      }
    );
    
    if (response.data.alerts_sent > 0) {
      console.log(`⚠️ ${response.data.alerts_sent} usage alerts sent!`);
      response.data.alerts.forEach(alert => {
        console.log(`Alert: ${alert.type} - ${alert.threshold_percent}% used`);
      });
    }
  } catch (error) {
    console.error('Failed to check alerts:', error.message);
  }
}

// Check alerts after heavy usage
await heavyWorkload();
await checkMyUsageAlerts();
```

### Python - Monitor Alert History

```python
import requests
from datetime import datetime

def get_recent_alerts(api_key):
    """Fetch and display recent usage alerts"""
    response = requests.get(
        'https://api.nanolambda.com/usage/alerts',
        headers={'Authorization': f'Bearer {api_key}'}
    )
    
    if response.ok:
        data = response.json()
        alerts = data.get('alerts', [])
        
        print(f"Found {len(alerts)} alerts:\n")
        for alert in alerts:
            sent_time = datetime.fromtimestamp(alert['sent_at'])
            usage_pct = (alert['current_usage'] / alert['usage_limit']) * 100
            
            print(f"[{sent_time}] {alert['type'].upper()}")
            print(f"  Usage: {alert['current_usage']:,} / {alert['usage_limit']:,}")
            print(f"  Percentage: {usage_pct:.1f}%\n")
    else:
        print(f"Error: {response.status_code}")

# Run monthly report
get_recent_alerts(os.getenv('NANOLAMBDA_API_KEY'))
```

### Bash - Cron Job for Monitoring

```bash
#!/bin/bash
# check-usage-alerts.sh - Run via cron every hour

API_KEY="your_api_key_here"
WEBHOOK_URL="https://your-monitoring-service.com/webhook"

# Check for new alerts
response=$(curl -s -X POST \
  https://api.nanolambda.com/usage/check-alerts \
  -H "Authorization: Bearer $API_KEY")

# Parse alerts_sent count
alerts_sent=$(echo $response | jq -r '.alerts_sent')

# Send to monitoring service if alerts were triggered
if [ "$alerts_sent" -gt 0 ]; then
  curl -X POST $WEBHOOK_URL \
    -H "Content-Type: application/json" \
    -d "{\"message\": \"NanoLambda: $alerts_sent usage alerts triggered\", \"data\": $response}"
fi
```

Add to crontab:
```bash
# Check usage alerts every hour
0 * * * * /path/to/check-usage-alerts.sh >> /var/log/nanolambda-alerts.log 2>&1
```

## Troubleshooting

### Alerts Not Being Sent

**Problem**: No emails received despite reaching thresholds

**Solutions**:
1. **Check SMTP Configuration**
   ```bash
   # Test SMTP settings
   curl -X POST http://localhost:8080/usage/check-alerts \
     -H "Authorization: Bearer YOUR_KEY"
   
   # Check server logs for email errors
   tail -f /var/log/nanolambda.log | grep -i email
   ```

2. **Verify Email Address**
   - Ensure Stripe customer has valid email
   - Check spam/junk folders
   - Verify email deliverability with your provider

3. **Check Alert History**
   ```bash
   # Verify alerts were recorded
   curl -X GET http://localhost:8080/usage/alerts \
     -H "Authorization: Bearer YOUR_KEY"
   ```

### Duplicate Alerts

**Problem**: Receiving multiple alerts for same threshold

**Solutions**:
1. Check database for duplicate entries:
   ```sql
   SELECT * FROM usage_alerts 
   WHERE api_key = 'your_key' 
   ORDER BY sent_at DESC LIMIT 10;
   ```

2. Verify period_start consistency
3. Check for concurrent API calls triggering alerts

### Wrong Usage Percentage

**Problem**: Alert shows incorrect usage percentage

**Solutions**:
1. **Verify Tier Limits**
   ```bash
   curl http://localhost:8080/tier/current \
     -H "Authorization: Bearer YOUR_KEY"
   ```

2. **Check Usage Calculation**
   - Ensure monthly_invocations counter is accurate
   - Verify billing period start/end dates
   - Check for timezone issues in timestamp calculations

3. **Reset if Needed**
   ```sql
   -- Reset monthly counter (start of new billing period)
   UPDATE user_tiers 
   SET monthly_invocations = 0, month_start = strftime('%s', 'now')
   WHERE api_key = 'your_key';
   ```

### Email Not Configured

**Problem**: "Email service not configured" error in logs

**Solution**: Set up SMTP environment variables (see Configuration section above)

## Roadmap

Future enhancements planned:

- [ ] **Custom Thresholds**: Allow users to set their own alert percentages
- [ ] **SMS Alerts**: Add Twilio integration for text message notifications
- [ ] **Webhook Integration**: Send alerts to custom webhooks for integration
- [ ] **Slack/Discord**: Direct team notifications
- [ ] **Alert Scheduling**: Choose quiet hours for non-critical alerts
- [ ] **Multi-Channel**: Combine email + SMS + webhook for critical alerts
- [ ] **Predictive Alerts**: ML-based usage forecasting ("You'll hit 100% in 3 days")
- [ ] **Alert Preferences**: Per-user configuration of alert channels and thresholds
- [ ] **Snooze Alerts**: Temporarily disable alerts for maintenance periods
- [ ] **Alert Rules**: Complex conditions (e.g., "80% used AND 5 days remaining")

## Support

For questions or issues with usage alerts:

- **Documentation**: https://docs.nanolambda.com/usage-alerts
- **Email**: support@nanolambda.com
- **Community**: https://community.nanolambda.com
- **GitHub Issues**: https://github.com/nanolambda/nanolambda/issues

## Related Documentation

- [Email Notifications](./email-notifications.md) - SMTP setup and configuration
- [Customer Portal](./customer-portal.md) - Self-service billing management
- [Metered Billing](./metered-billing.md) - Usage tracking and overage charges
- [Tiered Pricing](./tiered-pricing.md) - Understanding tier limits and features
