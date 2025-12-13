# Stripe Payment Integration Setup Guide

## Overview

NanoLambda supports Stripe payment integration for managing paid tier subscriptions. This guide explains how to set up Stripe for your NanoLambda deployment.

## Current Status

**Note:** The current implementation is a **mock/simplified version** that demonstrates the payment flow structure. For production use, you'll need to integrate the real Stripe SDK.

## Prerequisites

1. A Stripe account (sign up at [stripe.com](https://stripe.com))
2. Stripe API keys (test mode for development)
3. Products and prices configured in Stripe Dashboard

## Setup Steps

### 1. Create Stripe Products

In your Stripe Dashboard (https://dashboard.stripe.com/products):

1. Create three products:
   - **Starter Plan**: $299/month
   - **Pro Plan**: $999/month  
   - **Enterprise Plan**: $2999/month

2. Create recurring monthly prices for each product

3. Copy the Price IDs (e.g., `price_1ABC...`)

### 2. Configure Environment Variables

Copy `.env.example` to `.env`:

```bash
cp .env.example .env
```

Fill in your Stripe credentials:

```env
# Get from https://dashboard.stripe.com/apikeys
STRIPE_SECRET_KEY=sk_test_51...

# Get from https://dashboard.stripe.com/webhooks
STRIPE_WEBHOOK_SECRET=whsec_...

# Price IDs from your products
STRIPE_PRICE_STARTER=price_1ABC...starter
STRIPE_PRICE_PRO=price_1ABC...pro
STRIPE_PRICE_ENTERPRISE=price_1ABC...enterprise
```

### 3. Set Up Webhooks

1. Go to https://dashboard.stripe.com/webhooks
2. Add an endpoint: `https://your-domain.com/webhooks/stripe`
3. Select events to listen to:
   - `customer.subscription.created`
   - `customer.subscription.updated`
   - `customer.subscription.deleted`
   - `payment_intent.succeeded`
   - `payment_intent.payment_failed`
4. Copy the webhook signing secret to `STRIPE_WEBHOOK_SECRET`

### 4. Test the Integration

#### Start the Server

```bash
STRIPE_SECRET_KEY=sk_test_... cargo run --bin nanolambda-server
```

#### Create an API Key

```bash
curl -X POST http://localhost:8080/auth/keys \
  -H "Content-Type: application/json" \
  -d '{"name": "test-key"}'
```

#### Create a Stripe Customer

```bash
curl -X POST http://localhost:8080/payment/customer \
  -H "X-API-Key: nl_your_key_here" \
  -H "Content-Type: application/json" \
  -d '{"email": "customer@example.com", "name": "Test Customer"}'
```

#### Attach a Payment Method

```bash
curl -X POST http://localhost:8080/payment/method \
  -H "X-API-Key: nl_your_key_here" \
  -H "Content-Type: application/json" \
  -d '{"payment_method_id": "pm_test_..."}'
```

#### Create a Subscription

```bash
curl -X POST http://localhost:8080/payment/subscription \
  -H "X-API-Key: nl_your_key_here" \
  -H "Content-Type: application/json" \
  -d '{"tier": "pro"}'
```

## Payment API Endpoints

### Protected Endpoints (require X-API-Key header)

#### POST /payment/customer
Create or retrieve Stripe customer for the authenticated user.

**Request:**
```json
{
  "email": "user@example.com",
  "name": "John Doe"
}
```

**Response:**
```json
{
  "success": true,
  "customer_id": "cus_...",
  "created_at": 1234567890
}
```

#### GET /payment/customer
Get customer payment information.

**Response:**
```json
{
  "customer_id": "cus_...",
  "subscription_id": "sub_...",
  "payment_method_id": "pm_...",
  "subscription_status": "active",
  "created_at": 1234567890,
  "updated_at": 1234567890
}
```

#### POST /payment/method
Attach a payment method to the customer.

**Request:**
```json
{
  "payment_method_id": "pm_card_..."
}
```

**Response:**
```json
{
  "success": true,
  "message": "Payment method attached successfully",
  "payment_method_id": "pm_card_..."
}
```

#### POST /payment/subscription
Create a subscription for a specific tier.

**Request:**
```json
{
  "tier": "pro"
}
```

**Response:**
```json
{
  "success": true,
  "subscription_id": "sub_...",
  "status": "active",
  "tier": "pro"
}
```

#### DELETE /payment/subscription
Cancel the active subscription.

**Response:**
```json
{
  "success": true,
  "message": "Subscription canceled successfully",
  "subscription_id": "sub_...",
  "status": "canceled"
}
```

### Public Endpoints

#### POST /webhooks/stripe
Stripe webhook handler (verified by signature).

Accepts webhook events from Stripe and updates subscription status accordingly.

## Production Integration

To upgrade to the real Stripe integration:

1. **Add Stripe SDK dependency:**
   ```toml
   # In crates/storage/Cargo.toml
   stripe = { version = "0.35", features = ["async", "webhook-events"] }
   ```

2. **Replace mock implementations in** `crates/storage/src/payment.rs`:
   - Use `stripe::Customer::create()` instead of mock customer creation
   - Use `stripe::PaymentMethod::attach()` for payment methods
   - Use `stripe::Subscription::create()` for subscriptions
   - Use `stripe::Webhook::construct_event()` for webhook verification

3. **Update error handling:**
   - Add proper retry logic for API failures
   - Handle rate limits and network errors
   - Add logging for payment events

4. **Security considerations:**
   - Store `STRIPE_SECRET_KEY` securely (e.g., AWS Secrets Manager, Vault)
   - Use HTTPS for webhook endpoint
   - Verify webhook signatures on every event
   - Implement idempotency for payment operations

## Payment Flow

1. **Trial Period** (automatic):
   - New users start with 14-day trial
   - 100,000 invocations included
   - No payment required

2. **Upgrade to Paid Tier**:
   - User creates Stripe customer record
   - Attaches payment method (credit card)
   - Creates subscription for chosen tier
   - System assigns tier and updates limits

3. **Subscription Management**:
   - Webhooks keep subscription status in sync
   - Failed payments trigger status updates
   - Canceled subscriptions revert to trial/free tier

4. **Enforcement**:
   - Monthly quotas enforced on invocations
   - Memory/timeout limits enforced per tier
   - HTTP 402 returned when payment required

## Troubleshooting

### "Payment processing not available"

- Check that `STRIPE_SECRET_KEY` environment variable is set
- Verify the key format is correct (`sk_test_...` or `sk_live_...`)
- Check server logs for initialization errors

### Webhook signature verification fails

- Ensure `STRIPE_WEBHOOK_SECRET` matches your webhook endpoint secret
- Check that webhook endpoint URL is correct in Stripe Dashboard
- Verify HTTPS is being used (required for webhooks)

### Subscription creation fails

- Confirm customer has a payment method attached
- Check that price IDs match your Stripe Dashboard products
- Review Stripe Dashboard logs for detailed error messages

## Support

For Stripe-specific issues:
- [Stripe Documentation](https://stripe.com/docs)
- [Stripe Support](https://support.stripe.com/)

For NanoLambda integration issues:
- Check server logs: `tail -f nanolambda.log`
- Enable debug logging: `RUST_LOG=debug cargo run`
- Review database: `sqlite3 nanolambda.db.usage.db`
