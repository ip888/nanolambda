# Stripe Payment Integration

## Overview

NanoLambda now has **real Stripe API integration** using direct HTTP calls via `reqwest`. This is not a mock - it makes actual API calls to Stripe.

## Status: ✅ Production Ready

**Implemented Features:**
- ✅ Customer creation with metadata
- ✅ Payment method attachment and setting as default
- ✅ Subscription creation with tier metadata
- ✅ Subscription updates (tier upgrades/downgrades with proration)
- ✅ Subscription cancellation
- ✅ Proper error handling with Stripe API errors
- ✅ SQLite persistence for customer/subscription data
- ⏳ Webhook signature verification (TODO - Priority)
- ⏳ Webhook event handlers (TODO - Priority)

## Architecture

```
┌─────────────────┐
│   NanoLambda    │
│   API Server    │
└────────┬────────┘
         │
         │ reqwest HTTP client
         │ Basic Auth: API key
         ▼
┌─────────────────┐
│  Stripe API     │
│  (api.stripe    │
│   .com/v1)      │
└─────────────────┘
```

**Technical Details:**
- HTTP client: `reqwest` with Basic Auth
- Authentication: Stripe secret key as username, empty password
- Request format: Form-encoded data
- Response format: JSON
- Database: SQLite for caching customer/subscription mappings

## Setup

### 1. Create Stripe Account

1. Sign up at [stripe.com](https://stripe.com)
2. Verify your email
3. Complete business profile (for production)

### 2. Get API Keys

Visit https://dashboard.stripe.com/apikeys

**Test Mode (Development):**
```
STRIPE_SECRET_KEY=sk_test_51ABC...your_test_key
```

**Live Mode (Production):**
```
STRIPE_SECRET_KEY=sk_live_51ABC...your_live_key
```

⚠️ **Never commit your secret keys to version control!**

### 3. Create Products & Prices

Go to https://dashboard.stripe.com/products

Create three products with monthly recurring prices:

| Product | Price/Month | Price ID | Tier |
|---------|-------------|----------|------|
| Starter | $299 | `price_starter_xxx` | starter |
| Pro | $999 | `price_pro_xxx` | pro |
| Enterprise | $2999 | `price_enterprise_xxx` | enterprise |

**Important:** Copy the Price IDs (not Product IDs)

### 4. Configure Environment

Edit `.env`:

```env
# Required: Stripe secret key
STRIPE_SECRET_KEY=sk_test_51ABC...

# Required: Price IDs from Stripe Dashboard
STRIPE_PRICE_STARTER=price_1ABC...starter
STRIPE_PRICE_PRO=price_1ABC...pro  
STRIPE_PRICE_ENTERPRISE=price_1ABC...enterprise

# Optional: Webhook secret (for production)
STRIPE_WEBHOOK_SECRET=whsec_...
```

### 5. Start Server

```bash
# Load environment variables
source .env

# Start server
cargo run --bin nanolambda-server --release
```

Server will log:
```
INFO nanolambda_api: Stripe payment integration enabled
INFO nanolambda_api: Starting API server on 0.0.0.0:8080
```

## API Usage

### Create Customer

Creates a Stripe customer and links it to your API key.

```bash
curl -X POST http://localhost:8080/payment/customer \
  -H "Authorization: Bearer nl_your_api_key" \
  -H "Content-Type: application/json" \
  -d '{
    "email": "customer@example.com",
    "name": "John Doe"
  }'
```

**Response:**
```json
{
  "success": true,
  "customer_id": "cus_ABC123",
  "created_at": 1765643064
}
```

**What happens:**
1. POST request to `https://api.stripe.com/v1/customers`
2. Customer created with email, name, and metadata (nanolambda_api_key)
3. Customer ID stored in SQLite database
4. Idempotent: Returns existing customer if already created

### Get Customer Info

```bash
curl http://localhost:8080/payment/customer \
  -H "Authorization: Bearer nl_your_api_key"
```

**Response:**
```json
{
  "api_key": "nl_...",
  "stripe_customer_id": "cus_ABC123",
  "stripe_subscription_id": "sub_XYZ789",
  "payment_method_id": "pm_card_visa",
  "subscription_status": "active",
  "created_at": 1765643064,
  "updated_at": 1765643120
}
```

### Attach Payment Method

Attaches a payment method and sets it as the default for the customer.

```bash
curl -X POST http://localhost:8080/payment/method \
  -H "Authorization: Bearer nl_your_api_key" \
  -H "Content-Type: application/json" \
  -d '{
    "payment_method_id": "pm_card_visa"
  }'
```

**Test Payment Methods** (Stripe test mode):
- `pm_card_visa`: Visa ending in 4242
- `pm_card_mastercard`: Mastercard
- Create via Stripe Dashboard or use stripe.js on frontend

**What happens:**
1. POST to `/payment_methods/{id}/attach` with customer ID
2. POST to `/customers/{id}` to set as default payment method
3. Database updated with payment method ID

### Create Subscription

Creates a subscription for a specific tier.

```bash
curl -X POST http://localhost:8080/payment/subscription \
  -H "Authorization: Bearer nl_your_api_key" \
  -H "Content-Type: application/json" \
  -d '{
    "tier": "pro"
  }'
```

**Response:**
```json
{
  "success": true,
  "subscription_id": "sub_ABC123",
  "status": "active",
  "tier": "pro"
}
```

**Requirements:**
- Customer must exist
- Payment method must be attached
- Tier must be: `starter`, `pro`, or `enterprise`

**What happens:**
1. Validates customer and payment method
2. POST to `/subscriptions` with customer, price_id, and metadata
3. Stripe immediately charges the customer
4. Database updated with subscription ID and status

### Cancel Subscription

```bash
curl -X DELETE http://localhost:8080/payment/subscription \
  -H "Authorization: Bearer nl_your_api_key"
```

**Response:**
```json
{
  "success": true,
  "subscription_id": "sub_ABC123",
  "status": "canceled"
}
```

**What happens:**
1. DELETE to `/subscriptions/{id}`
2. Subscription canceled immediately (no refund by default)
3. Database updated with canceled status
4. User retains access until period end (configurable)

## Testing

### Test with Stripe Test Mode

All test cards: https://stripe.com/docs/testing

```bash
# 1. Create API key
API_KEY=$(curl -s -X POST http://localhost:8080/auth/keys \
  -H "Content-Type: application/json" \
  -d '{"name":"test-payment"}' | jq -r '.key')

echo "API Key: $API_KEY"

# 2. Create customer
curl -X POST http://localhost:8080/payment/customer \
  -H "Authorization: Bearer $API_KEY" \
  -H "Content-Type: application/json" \
  -d '{"email":"test@example.com","name":"Test User"}' | jq

# 3. Get payment method from Stripe Dashboard or stripe.js
# For testing, use Stripe Dashboard > Payments > Payment Methods
PM_ID="pm_card_visa"  # Replace with real test payment method

# 4. Attach payment method
curl -X POST http://localhost:8080/payment/method \
  -H "Authorization: Bearer $API_KEY" \
  -H "Content-Type: application/json" \
  -d "{\"payment_method_id\":\"$PM_ID\"}" | jq

# 5. Create subscription
curl -X POST http://localhost:8080/payment/subscription \
  -H "Authorization: Bearer $API_KEY" \
  -H "Content-Type: application/json" \
  -d '{"tier":"pro"}' | jq

# 6. Verify in Stripe Dashboard
# Visit: https://dashboard.stripe.com/subscriptions
```

### Verify in Stripe Dashboard

After running the API calls:

1. Go to https://dashboard.stripe.com/customers
2. Find your customer (search by email)
3. Check **Subscriptions** tab - should show active subscription
4. Check **Payment methods** tab - should show attached card
5. View **Metadata** - should see `nanolambda_api_key` and `nanolambda_tier`

## Error Handling

**Common Errors:**

| Error | Status | Meaning |
|-------|--------|---------|
| `PaymentUnavailable` | 503 | `STRIPE_SECRET_KEY` not set |
| `CustomerCreationFailed` | 500 | Stripe API error (check logs) |
| `Customer not found` | 400 | Create customer first |
| `No payment method attached` | 400 | Attach payment method first |
| `Invalid tier` | 400 | Tier must be starter/pro/enterprise |
| `Stripe API error (401)` | 500 | Invalid API key |
| `Stripe API error (402)` | 500 | Payment failed |

**Check Logs:**
```bash
# Server logs show full Stripe API responses
tail -f /path/to/server.log | grep -i stripe
```

## Production Deployment

### Security Checklist

- [ ] Use `STRIPE_SECRET_KEY` for live mode (starts with `sk_live_`)
- [ ] Never expose secret key in client-side code
- [ ] Enable HTTPS (Stripe requires it for webhooks)
- [ ] Set up webhook endpoint and verify signatures
- [ ] Use environment variables (never hardcode keys)
- [ ] Rotate keys periodically
- [ ] Enable Stripe Radar for fraud detection
- [ ] Set up proper error monitoring

### Webhook Setup

1. Go to https://dashboard.stripe.com/webhooks
2. Add endpoint: `https://yourdomain.com/webhooks/stripe`
3. Select events:
   - `customer.subscription.created`
   - `customer.subscription.updated`
   - `customer.subscription.deleted`
   - `invoice.payment_succeeded`
   - `invoice.payment_failed`
   - `payment_method.attached`
   - `payment_method.detached`
4. Copy webhook signing secret to `STRIPE_WEBHOOK_SECRET`

**Webhook Verification (TODO):**
```rust
// Coming soon: webhook signature verification
// Will use HMAC SHA-256 with STRIPE_WEBHOOK_SECRET
```

### Monitoring

Track these metrics:

- Successful subscriptions created/day
- Failed payment attempts
- Subscription churn rate
- Average revenue per user (ARPU)
- Payment method failures

**Stripe Dashboard Analytics:**
- https://dashboard.stripe.com/analytics
- Revenue charts
- Subscription metrics
- Payment success rates

## Architecture Notes

### Why reqwest Instead of async-stripe?

1. **Stability**: Direct HTTP calls avoid SDK version conflicts
2. **Control**: Full control over request/response handling
3. **Simplicity**: Minimal dependencies
4. **Flexibility**: Easy to add custom fields or metadata
5. **Debugging**: Clear visibility into API calls

### Database Schema

```sql
CREATE TABLE stripe_customers (
    api_key TEXT PRIMARY KEY,
    stripe_customer_id TEXT NOT NULL UNIQUE,
    stripe_subscription_id TEXT,
    payment_method_id TEXT,
    subscription_status TEXT,
    created_at INTEGER NOT NULL,
    updated_at INTEGER NOT NULL
);
```

**Why cache in SQLite?**
- Fast lookups without API calls
- Reduces Stripe API quota usage
- Enables offline operation (read-only)
- Provides audit trail

### Idempotency

All operations are idempotent:
- Creating customer twice returns existing customer
- Attaching payment method overwrites previous
- Creating subscription fails if one exists (by design)

## Troubleshooting

### "Payment processing not available"

```bash
# Check if STRIPE_SECRET_KEY is set
echo $STRIPE_SECRET_KEY

# Should start with sk_test_ or sk_live_
# If empty, add to .env and restart server
```

### "Stripe API error (401): Unauthorized"

API key is invalid or not set correctly.

```bash
# Verify key in Stripe Dashboard
https://dashboard.stripe.com/apikeys

# Make sure you're using the SECRET key, not PUBLISHABLE key
# Secret key starts with: sk_test_ or sk_live_
# Publishable starts with: pk_test_ or pk_live_ (DON'T use this!)
```

### "No payment method attached"

Payment method must be attached before creating subscription.

**Fix:**
1. Create payment method in Stripe Dashboard (test mode)
2. Copy payment method ID (starts with `pm_`)
3. Call `/payment/method` endpoint
4. Then retry subscription creation

### "Could not parse Stripe response"

Stripe API response structure changed or incomplete.

**Debug:**
```bash
# Enable debug logging
RUST_LOG=debug cargo run --bin nanolambda-server
```

Check response structure in logs and update struct definitions if needed.

## Next Steps

1. **Implement Webhooks** (Priority 1)
   - Signature verification
   - Event handlers for subscription updates
   - Automatic tier adjustment on payment failure

2. **Add Payment UI** (Priority 2)
   - Stripe Elements integration
   - Payment method management page
   - Subscription upgrade/downgrade UI

3. **Billing Portal** (Priority 3)
   - Let customers manage their subscriptions
   - Update payment methods
   - View invoices
   - Use Stripe Customer Portal

4. **Usage-Based Billing** (Future)
   - Report usage to Stripe
   - Metered billing for overages
   - Usage caps and alerts

## Resources

- [Stripe API Docs](https://stripe.com/docs/api)
- [Stripe Testing](https://stripe.com/docs/testing)
- [Stripe Webhooks Guide](https://stripe.com/docs/webhooks)
- [reqwest Documentation](https://docs.rs/reqwest)
- [NanoLambda GitHub](https://github.com/ip888/nanolambda)

## Support

For issues or questions:
- GitHub Issues: https://github.com/ip888/nanolambda/issues
- Stripe Support: https://support.stripe.com
