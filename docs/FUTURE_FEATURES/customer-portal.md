# Customer Portal Integration

## Overview

The Customer Portal integration allows customers to self-manage their subscriptions, payment methods, and billing information through Stripe's hosted portal. This significantly reduces support burden by empowering customers to handle common billing tasks themselves.

## Key Features

- **Self-Service Subscription Management**: Customers can upgrade, downgrade, or cancel subscriptions
- **Payment Method Updates**: Securely update credit cards and payment methods
- **Invoice Access**: View and download past invoices
- **Billing History**: See complete payment history
- **One-Click Access**: Seamless redirect from dashboard
- **Secure & Branded**: Hosted by Stripe with your branding

## Configuration

### Stripe Dashboard Setup

1. **Enable Customer Portal**:
   - Go to [Stripe Dashboard](https://dashboard.stripe.com) → Settings → Billing → Customer portal
   - Click "Activate test mode link" (for testing)
   - Configure portal settings

2. **Configure Portal Settings**:
   - **Products**: Select which products customers can subscribe to
   - **Features**: Enable/disable subscription cancellation, plan changes, etc.
   - **Business information**: Add company name, support email, terms of service
   - **Branding**: Upload logo, set brand colors
   - **Functionality**: Configure what customers can do:
     - ✅ Update payment method
     - ✅ View invoices
     - ✅ Cancel subscription
     - ✅ Change plan (upgrade/downgrade)
     - ✅ View billing history

3. **Portal URL Structure**:
   ```
   https://billing.stripe.com/p/session/{session_id}
   ```

### Environment Variables

No additional environment variables required beyond existing Stripe configuration:

```bash
STRIPE_SECRET_KEY=sk_test_...  # or sk_live_... for production
```

## API Usage

### Create Portal Session

**Endpoint**: `POST /payment/portal`

**Headers**:
```
Authorization: Bearer {api_key}
Content-Type: application/json
```

**Request Body** (Optional):
```json
{
  "return_url": "https://nanolambda.com/dashboard"
}
```

**Response** (Success):
```json
{
  "success": true,
  "portal_url": "https://billing.stripe.com/p/session/abc123...",
  "session_id": "bps_abc123..."
}
```

**Response** (Error):
```json
{
  "error": "PortalSessionFailed",
  "message": "Failed to create portal session: Customer not found"
}
```

### Example: Redirect User to Portal

```bash
curl -X POST http://localhost:8080/payment/portal \
  -H "Authorization: Bearer your-api-key" \
  -H "Content-Type: application/json" \
  -d '{"return_url": "https://nanolambda.com/dashboard"}'
```

Response:
```json
{
  "success": true,
  "portal_url": "https://billing.stripe.com/p/session/bps_1234...",
  "session_id": "bps_1234..."
}
```

Then redirect user to `portal_url`:
```javascript
window.location.href = response.portal_url;
// or open in new tab
window.open(response.portal_url, '_blank');
```

## Dashboard Integration

The dashboard includes a "⚙️ Manage Billing" button in the Current Tier card that:

1. Calls `/payment/portal` API endpoint
2. Opens returned portal URL in new tab
3. Customer manages billing in Stripe portal
4. Returns to dashboard via `return_url`

### User Flow

```
Dashboard → Click "Manage Billing"
    ↓
JavaScript calls /payment/portal API
    ↓
API creates Stripe Portal session
    ↓
Portal URL returned to browser
    ↓
New tab opens with Stripe Portal
    ↓
Customer manages subscription/payment
    ↓
Customer clicks "Return to {your-app}" button
    ↓
Redirected back to return_url (dashboard)
```

## Code Implementation

### Backend: Create Portal Session

```rust
// In PaymentManager (crates/storage/src/payment.rs)
pub async fn create_portal_session(
    &self,
    api_key: &str,
    return_url: Option<&str>,
) -> Result<PortalSession> {
    let customer = self.get_customer(api_key).await?
        .ok_or_else(|| anyhow!("Customer not found"))?;

    let mut form = vec![
        ("customer", customer.stripe_customer_id.clone()),
    ];

    if let Some(url) = return_url {
        form.push(("return_url", url.to_string()));
    }

    let session: PortalSession = self.stripe_api_call(
        "POST",
        "/billing_portal/sessions",
        Some(&form),
    ).await?;

    Ok(session)
}
```

### Frontend: Open Portal

```javascript
async function openCustomerPortal() {
    const apiKey = document.getElementById('apiKeyInput').value;
    
    const response = await fetch('/payment/portal', {
        method: 'POST',
        headers: {
            'Content-Type': 'application/json',
            'Authorization': `Bearer ${apiKey}`
        },
        body: JSON.stringify({
            return_url: window.location.href
        })
    });
    
    const data = await response.json();
    if (data.success && data.portal_url) {
        window.open(data.portal_url, '_blank');
    }
}
```

## Portal Capabilities

### What Customers Can Do

1. **View Subscription Details**:
   - Current plan and pricing
   - Next billing date
   - Billing cycle
   - Subscription status

2. **Update Payment Method**:
   - Add new credit/debit card
   - Remove old payment methods
   - Set default payment method

3. **View Invoices**:
   - Download PDF invoices
   - View payment history
   - See upcoming invoices

4. **Change Subscription**:
   - Upgrade to higher tier
   - Downgrade to lower tier
   - View plan comparison

5. **Cancel Subscription**:
   - Cancel immediately or at period end
   - View cancellation terms
   - Reactivate before period ends

6. **Update Billing Email**:
   - Change email for invoices
   - Update contact information

### What Customers CANNOT Do

Portal restrictions (configurable in Stripe Dashboard):
- Cannot change past invoices
- Cannot refund themselves
- Cannot access other customers' data
- Cannot modify subscription start dates
- Cannot apply custom discounts

## Security

### Session Security

- Portal sessions are **single-use**
- Sessions expire after **24 hours** if not used
- Sessions expire **4 hours** after first access
- Each session tied to specific customer
- Cannot be reused or shared

### Customer Verification

- Portal only accessible via authenticated session
- Customer must have valid Stripe customer ID
- API key required to create portal session
- Return URL validated by Stripe

### Data Protection

- All data transmitted over HTTPS
- Payment details never exposed to our servers
- PCI compliance handled by Stripe
- No storage of payment card numbers

## Error Handling

### Common Errors

**"Customer not found"**:
- Customer hasn't been created in Stripe yet
- Solution: Call `POST /payment/customer` first

**"Payment processing not available"**:
- `STRIPE_SECRET_KEY` not configured
- Solution: Set environment variable

**"Failed to create portal session"**:
- Network error or Stripe API issue
- Solution: Retry or check Stripe Dashboard

### Error Response Format

```json
{
  "error": "ErrorCode",
  "message": "Human-readable error description"
}
```

### User-Friendly Error Messages

```javascript
try {
    // Open portal
} catch (error) {
    alert(`Failed to open billing portal: ${error.message}\n\n` +
          `Make sure you have an active subscription and payment method.`);
}
```

## Testing

### Manual Testing

1. **Create a test customer**:
   ```bash
   curl -X POST http://localhost:8080/payment/customer \
     -H "Authorization: Bearer test-api-key" \
     -H "Content-Type: application/json" \
     -d '{"email": "test@example.com"}'
   ```

2. **Attach test payment method**:
   ```bash
   curl -X POST http://localhost:8080/payment/method \
     -H "Authorization: Bearer test-api-key" \
     -H "Content-Type: application/json" \
     -d '{"payment_method_id": "pm_card_visa"}'
   ```

3. **Create test subscription**:
   ```bash
   curl -X POST http://localhost:8080/payment/subscription \
     -H "Authorization: Bearer test-api-key" \
     -H "Content-Type: application/json" \
     -d '{"tier": "starter"}'
   ```

4. **Open portal**:
   ```bash
   curl -X POST http://localhost:8080/payment/portal \
     -H "Authorization: Bearer test-api-key" \
     -H "Content-Type: application/json" \
     -d '{"return_url": "http://localhost:8080/dashboard"}'
   ```

5. **Visit portal URL**:
   - Copy `portal_url` from response
   - Open in browser
   - Test subscription management

### Stripe Test Cards

Use these test cards in the portal:

| Card Number         | Scenario                    |
|---------------------|----------------------------|
| 4242 4242 4242 4242 | Successful payment         |
| 4000 0000 0000 9995 | Insufficient funds decline |
| 4000 0000 0000 0002 | Card declined              |
| 4000 0025 0000 3155 | Requires authentication    |

Use any future expiry date, any CVC.

### Test Scenarios

1. **✅ Portal Access**: Verify portal loads correctly
2. **✅ View Subscription**: Check current plan displays
3. **✅ Update Payment**: Add new test card
4. **✅ Change Plan**: Upgrade from Starter to Pro
5. **✅ View Invoices**: Download test invoice PDF
6. **✅ Cancel Subscription**: Test cancellation flow
7. **✅ Return URL**: Verify redirect back to dashboard

## Best Practices

### Return URLs

- **Always provide return_url** for better UX
- Use full URLs (not relative): `https://app.com/dashboard`
- Include query params if needed: `?session=restored`
- Validate return URLs are on your domain (Stripe does this)

### User Experience

- **Open in new tab** (`_blank`): User can keep working
- **Show loading state**: Portal takes 1-2 seconds to load
- **Handle errors gracefully**: Show helpful messages
- **Test on mobile**: Portal is mobile-responsive

### Portal Branding

- Upload your logo in Stripe Dashboard
- Use brand colors for consistency
- Add support email for customer questions
- Include terms of service and privacy policy links

## Troubleshooting

### Portal won't open

**Symptoms**: Error creating portal session

**Checks**:
1. Customer exists in Stripe
2. `STRIPE_SECRET_KEY` is set
3. API key is valid and authorized
4. Customer has been created via `/payment/customer`

**Fix**:
```bash
# Check if customer exists
curl -X GET http://localhost:8080/payment/customer \
  -H "Authorization: Bearer your-api-key"
```

### Portal shows "No subscriptions"

**Cause**: Customer has no active subscriptions

**Fix**: Create a subscription first via `/payment/subscription`

### Return URL not working

**Cause**: Invalid or blocked return URL

**Fix**:
- Use full URLs with protocol: `https://example.com/path`
- Ensure URL is on your domain
- Check Stripe Dashboard → Customer portal → Settings

### Portal session expired

**Cause**: Portal sessions expire after 24 hours

**Fix**: Create a new session (users should do this each time)

## Monitoring

### Logs

Portal session creation is logged:

```
INFO  Created portal session bps_123 for customer cus_456
```

Enable debug logging:
```bash
RUST_LOG=debug ./target/release/nanolambda-server
```

### Metrics to Track

- Portal session creation rate
- Portal session click-through rate
- Actions taken in portal (upgrades, cancellations)
- Return rate after portal visit
- Support tickets reduced

### Stripe Dashboard

View portal usage:
1. Go to Stripe Dashboard → Billing → Customer portal
2. See usage statistics
3. Monitor customer actions
4. Review session logs

## Production Checklist

- [ ] Configure portal in Stripe Dashboard (live mode)
- [ ] Add company logo and branding
- [ ] Set support email address
- [ ] Add terms of service URL
- [ ] Add privacy policy URL
- [ ] Enable desired features (cancel, upgrade, etc.)
- [ ] Test portal in live mode with real payment
- [ ] Add portal link to main navigation
- [ ] Monitor portal usage metrics
- [ ] Train support team on portal capabilities

## Future Enhancements

- [ ] Embed portal inline (iframe) instead of redirect
- [ ] Add preview of portal in dashboard
- [ ] Track portal analytics (opens, actions taken)
- [ ] Custom portal branding per tier
- [ ] Portal access from email notifications
- [ ] Mobile app deep linking to portal
- [ ] Usage alerts with portal link
- [ ] Scheduled portal reminders (before renewal)

## Resources

- [Stripe Customer Portal Documentation](https://stripe.com/docs/billing/subscriptions/customer-portal)
- [Customer Portal Configuration](https://dashboard.stripe.com/settings/billing/portal)
- [Portal Session API](https://stripe.com/docs/api/customer_portal/session)
- [Portal Testing Guide](https://stripe.com/docs/testing)
