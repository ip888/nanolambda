# Metered Billing Implementation

## Overview

The metered billing system enables usage-based charging for customers who exceed their tier limits. This allows customers to scale beyond their subscription tier without hitting hard limits, while ensuring revenue captures actual usage.

## Key Features

- **Automatic Usage Reporting**: Report usage to Stripe's metered billing API
- **Overage Calculation**: Calculate costs for usage beyond tier limits
- **Database Tracking**: Local audit trail of all usage reports
- **Configurable Pricing**: Per-tier overage pricing (default: $0.20/1M for Starter)
- **Real-time Analysis**: API endpoints for usage and cost queries

## Architecture

### Components

1. **PaymentManager** (`crates/storage/src/payment.rs`)
   - `report_usage()`: Reports usage to Stripe and stores locally
   - `calculate_overage_cost()`: Calculates charges for overages
   - `create_subscription_with_metering()`: Creates subscriptions with metered pricing

2. **API Handlers** (`crates/api-server/src/handlers.rs`)
   - `report_metered_usage()`: POST endpoint for reporting usage
   - `calculate_overage()`: GET endpoint for overage analysis

3. **Database Tables**
   - `metered_usage_records`: Tracks all usage reports
     - `api_key`: User identifier
     - `stripe_usage_record_id`: Stripe's usage record ID
     - `quantity`: Number of invocations reported
     - `timestamp`: When invocations occurred
     - `reported_at`: When report was sent to Stripe
     - `period_start`, `period_end`: Billing period bounds

### Data Flow

```
User Invocation
    ↓
Usage Tracking (existing)
    ↓
[Optional] Report Usage API
    ↓
PaymentManager.report_usage()
    ↓
├─→ Stripe API: POST /subscription_items/{id}/usage_records
│   └─→ Response: usage_record_id
└─→ Database: INSERT into metered_usage_records
    ↓
Success Response
```

## Configuration

### Environment Variables

```bash
# Required for metered billing
STRIPE_SECRET_KEY=sk_test_...
STRIPE_METERED_PRICE_ID=price_...  # Stripe price with usage_type=metered

# Optional: Override default overage pricing
OVERAGE_PRICE_PER_INVOCATION=0.00002  # $0.20 per 1M invocations
```

### Stripe Setup

1. **Create Metered Price**:
   ```bash
   stripe prices create \
     --product prod_... \
     --currency usd \
     --billing_scheme per_unit \
     --unit_amount_decimal 0.0002 \
     --recurring[interval]=month \
     --recurring[usage_type]=metered \
     --recurring[aggregate_usage]=sum
   ```

2. **Create Subscription with Metering**:
   ```rust
   payment_mgr.create_subscription_with_metering(
       api_key,
       "price_starter_fixed",      // Fixed monthly price
       "price_metered_invocations"  // Metered overage price
   ).await?
   ```

## API Usage

### Report Usage

**Endpoint**: `POST /payment/usage`

**Headers**:
```
Authorization: Bearer {api_key}
Content-Type: application/json
```

**Request Body**:
```json
{
  "quantity": 50000,
  "timestamp": 1735603200  // Optional: Unix timestamp
}
```

**Response** (Success):
```json
{
  "success": true,
  "usage_record_id": "mbur_...",
  "quantity": 50000,
  "timestamp": 1735603200
}
```

**Response** (Error):
```json
{
  "error": "PaymentNotConfigured",
  "message": "Payment management not available. Set STRIPE_SECRET_KEY."
}
```

### Calculate Overage

**Endpoint**: `GET /payment/overage`

**Headers**:
```
Authorization: Bearer {api_key}
```

**Response** (Success):
```json
{
  "success": true,
  "current_usage": 1250000,
  "tier_limit": 1000000,
  "overage_invocations": 250000,
  "overage_cost": 0.0500,
  "currency": "usd"
}
```

**Response** (No Overage):
```json
{
  "success": true,
  "current_usage": 750000,
  "tier_limit": 1000000,
  "overage_invocations": 0,
  "overage_cost": 0.0000,
  "currency": "usd"
}
```

## Pricing Structure

### Per-Tier Overage Rates

| Tier       | Monthly Limit | Overage Rate       | Example Cost (100k over) |
|------------|---------------|--------------------|--------------------------|
| Starter    | 1M            | $0.20/1M invokes   | $0.02                    |
| Pro        | 10M           | $0.16/1M invokes   | $0.016                   |
| Enterprise | Unlimited     | N/A                | N/A                      |

### Cost Formula

```
overage_cost = (current_usage - tier_limit) × overage_price_per_invocation
```

Example for Starter tier:
- Current usage: 1,250,000 invocations
- Tier limit: 1,000,000 invocations
- Overage: 250,000 invocations
- Cost: 250,000 × $0.00002 = $0.05

## Database Schema

```sql
CREATE TABLE metered_usage_records (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    api_key TEXT NOT NULL,
    stripe_usage_record_id TEXT NOT NULL,
    quantity INTEGER NOT NULL,
    timestamp INTEGER NOT NULL,
    reported_at INTEGER NOT NULL,
    period_start INTEGER,
    period_end INTEGER
);

CREATE INDEX idx_metered_usage_api_key_timestamp 
ON metered_usage_records(api_key, timestamp);
```

## Testing

### Manual Testing

```bash
# Start server with metered billing enabled
export STRIPE_SECRET_KEY=sk_test_...
export STRIPE_METERED_PRICE_ID=price_...
./target/release/nanolambda-server

# Report usage
curl -X POST http://localhost:8080/payment/usage \
  -H "Authorization: Bearer test-api-key" \
  -H "Content-Type: application/json" \
  -d '{"quantity": 50000}'

# Check overage
curl -X GET http://localhost:8080/payment/overage \
  -H "Authorization: Bearer test-api-key"
```

### Automated Testing

```bash
# Run test suite
./test-metered-billing.py
```

The test script covers:
- ✅ Low usage (no overage)
- ✅ Medium usage (50% of limit)
- ✅ Near limit (90%)
- ✅ At limit (100%)
- ✅ Over limit (110%)
- ✅ Significant overage (150%)
- ✅ Current overage check

## Integration Points

### With Existing Systems

1. **Usage Tracking**: Connect to existing `usage_stats` table
   ```rust
   let usage = usage_db.get_monthly_invocations(api_key).await?;
   payment_mgr.report_usage(api_key, usage, None).await?;
   ```

2. **Tier Management**: Query tier limits for overage calculation
   ```rust
   let user_tier = tier_mgr.get_user_tier(api_key).await?;
   let tier_config = tier_mgr.get_tier_config(user_tier.tier).await;
   let overage = calculate_overage_cost(
       usage, 
       tier_config.max_invocations_per_month
   ).await?;
   ```

3. **Webhooks**: Process payment success events
   ```rust
   // In webhook handler for invoice.payment_succeeded
   if invoice.amount_paid > 0 {
       log::info!("Overage charge processed: ${}", invoice.amount_paid / 100);
   }
   ```

## Best Practices

### Reporting Frequency

- **Real-time**: Report usage immediately for critical limits
- **Batched**: Report hourly/daily for cost optimization
- **Monthly**: Report at billing period end for accuracy

### Error Handling

```rust
match payment_mgr.report_usage(api_key, quantity, None).await {
    Ok(record) => {
        log::info!("Usage reported: {}", record.id);
    }
    Err(e) => {
        // Log error but don't block execution
        log::error!("Failed to report usage: {}", e);
        // Queue for retry
    }
}
```

### Cost Optimization

1. **Batch Reports**: Combine multiple small reports into larger ones
2. **Cache Results**: Cache overage calculations for 5-10 minutes
3. **Async Processing**: Report usage asynchronously to avoid blocking

## Troubleshooting

### Common Issues

**Issue**: "Payment management not available"
- **Cause**: `STRIPE_SECRET_KEY` not set
- **Fix**: Set environment variable before starting server

**Issue**: "Metered price not configured"
- **Cause**: `STRIPE_METERED_PRICE_ID` not set
- **Fix**: Create metered price in Stripe and set environment variable

**Issue**: "Subscription item not found"
- **Cause**: Customer doesn't have subscription with metered pricing
- **Fix**: Create subscription using `create_subscription_with_metering()`

**Issue**: Usage reports not appearing in Stripe
- **Cause**: Incorrect API key or price ID
- **Fix**: Verify Stripe credentials and price configuration

### Debug Logging

```bash
# Enable debug logging
RUST_LOG=debug ./target/release/nanolambda-server

# Check database records
sqlite3 nanolambda.db "SELECT * FROM metered_usage_records ORDER BY reported_at DESC LIMIT 10"

# Verify Stripe records
stripe subscription_items list --subscription sub_...
```

## Future Enhancements

- [ ] Automatic usage reporting (scheduled task)
- [ ] Usage alerts at 80%/90%/100% of limit
- [ ] Historical usage analytics
- [ ] Per-function metered billing
- [ ] Tiered overage rates (first 100k @ $0.20, next @ $0.15, etc.)
- [ ] Grace period before charging overages
- [ ] Usage forecast/predictions

## References

- [Stripe Metered Billing Documentation](https://stripe.com/docs/billing/subscriptions/usage-based)
- [Stripe Usage Records API](https://stripe.com/docs/api/usage_records)
- AWS Lambda Pricing Model (inspiration)
