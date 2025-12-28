# Email Notifications System

## Overview

The email notification system automatically sends emails to customers for important payment and subscription events. Emails are sent via SMTP (compatible with SendGrid, AWS SES, Gmail, and other SMTP providers).

## Key Features

- **Automatic Notifications**: Triggered by Stripe webhook events
- **Multiple Event Types**: Payment success/failure, subscription changes, trial ending
- **HTML & Text Emails**: Both HTML and plain text versions for compatibility
- **Configurable SMTP**: Works with any SMTP provider
- **Non-Blocking**: Email failures don't block webhook processing
- **Professional Templates**: Clean, branded email templates

## Supported Email Events

### 1. Payment Succeeded
**Trigger**: Stripe `invoice.payment_succeeded` webhook  
**When**: Payment successfully processed  
**Content**:
- Payment amount
- Invoice number
- Payment date
- Thank you message

**Subject**: "Payment Successful - NanoLambda"

---

### 2. Payment Failed
**Trigger**: Stripe `invoice.payment_failed` webhook  
**When**: Payment declined or failed  
**Content**:
- Failed amount
- Failure reason
- Call-to-action: Update payment method
- Link to billing portal

**Subject**: "Payment Failed - Action Required"

---

### 3. Subscription Created
**Trigger**: Stripe `customer.subscription.created` webhook  
**When**: New subscription activated  
**Content**:
- Welcome message
- Plan/tier name
- Subscription ID
- Getting started steps
- Dashboard link

**Subject**: "Welcome to NanoLambda {Tier} Plan!"

---

### 4. Subscription Updated
**Trigger**: Stripe `customer.subscription.updated` webhook  
**When**: Subscription plan or status changes  
**Content**:
- Plan name
- New status
- Update confirmation

**Subject**: "Subscription Updated - NanoLambda"

---

### 5. Subscription Canceled
**Trigger**: Stripe `customer.subscription.deleted` webhook  
**When**: Subscription canceled  
**Content**:
- Cancellation confirmation
- Access end date
- Feedback request
- Reactivation option

**Subject**: "Subscription Canceled - NanoLambda"

---

### 6. Payment Method Attached
**Trigger**: Stripe `payment_method.attached` webhook  
**When**: Payment method added/updated  
**Content**:
- Card brand (Visa, Mastercard, etc.)
- Last 4 digits
- Confirmation message

**Subject**: "Payment Method Updated - NanoLambda"

---

### 7. Trial Ending
**Trigger**: Manual/scheduled notification (not yet implemented)  
**When**: Trial ending soon (e.g., 3 days before)  
**Content**:
- Days remaining
- Plan options and pricing
- Call-to-action: Choose a plan
- Billing portal link

**Subject**: "Trial Ending in {N} Days - NanoLambda"

## Configuration

### Environment Variables

```bash
# SMTP Server Configuration
SMTP_HOST=smtp.sendgrid.net          # SMTP server hostname
SMTP_PORT=587                        # SMTP port (default: 587 for TLS)
SMTP_USERNAME=apikey                 # SMTP username (SendGrid: "apikey")
SMTP_PASSWORD=SG.xxx                 # SMTP password/API key

# Email Sender Information
FROM_EMAIL=noreply@nanolambda.com    # From email address
FROM_NAME=NanoLambda                 # From name
```

### Provider-Specific Setup

#### SendGrid
```bash
SMTP_HOST=smtp.sendgrid.net
SMTP_PORT=587
SMTP_USERNAME=apikey
SMTP_PASSWORD=SG.your-api-key-here
FROM_EMAIL=noreply@yourdomain.com
FROM_NAME="NanoLambda"
```

**Setup Steps**:
1. Create SendGrid account: https://sendgrid.com
2. Generate API key: Settings → API Keys
3. Verify sender domain: Settings → Sender Authentication
4. Set environment variables above

#### AWS SES
```bash
SMTP_HOST=email-smtp.us-east-1.amazonaws.com
SMTP_PORT=587
SMTP_USERNAME=your-smtp-username
SMTP_PASSWORD=your-smtp-password
FROM_EMAIL=verified@yourdomain.com
FROM_NAME="NanoLambda"
```

**Setup Steps**:
1. Verify email/domain in SES console
2. Create SMTP credentials
3. Move out of sandbox (for production)
4. Set environment variables above

#### Gmail (Development Only)
```bash
SMTP_HOST=smtp.gmail.com
SMTP_PORT=587
SMTP_USERNAME=your-email@gmail.com
SMTP_PASSWORD=your-app-password
FROM_EMAIL=your-email@gmail.com
FROM_NAME="NanoLambda Dev"
```

**Setup Steps**:
1. Enable 2FA on Gmail account
2. Generate App Password: https://myaccount.google.com/apppasswords
3. Use app password (not regular password)
4. **Note**: Gmail has sending limits, not for production

## Architecture

### Email Flow

```
Stripe Webhook Event
    ↓
PaymentManager.handle_webhook()
    ↓
Event Handler (handle_payment_succeeded, etc.)
    ↓
├─→ Update Database
└─→ get_customer_email()
    ↓
send_email_notification()
    ↓
├─→ Build Email Template (HTML + Text)
└─→ EmailService.send_email()
    ↓
SMTP Server (SendGrid/SES/etc.)
    ↓
Customer's Inbox ✉️
```

### Components

1. **PaymentManager** (`crates/storage/src/payment.rs`)
   - `send_email_notification()`: Main email sending method
   - `get_customer_email()`: Fetch customer email from database/Stripe
   - Webhook handlers call email notifications

2. **EmailService** (`crates/storage/src/payment.rs`)
   - `from_env()`: Load SMTP configuration
   - `send_email()`: Send via SMTP using lettre crate
   - Supports HTML + plain text multipart emails

3. **Email Templates**
   - HTML templates with inline CSS
   - Plain text fallback for compatibility
   - Responsive design
   - Brand colors and styling

## Code Examples

### Manual Email Sending

```rust
use nanolambda_storage::payment::{PaymentManager, EmailEventType, EmailEventData};

// Send payment success email
let email_data = EmailEventData {
    amount: Some(1000), // $10.00 in cents
    invoice_id: Some("inv_123".to_string()),
    ..Default::default()
};

payment_mgr.send_email_notification(
    "customer@example.com",
    EmailEventType::PaymentSucceeded,
    email_data,
).await?;
```

### Webhook Integration (Automatic)

Email notifications are automatically sent by webhook handlers:

```rust
// In handle_payment_succeeded()
if let Ok(customer_email) = self.get_customer_email(&invoice.customer).await {
    let email_data = EmailEventData {
        amount: Some(invoice.amount_paid),
        invoice_id: Some(invoice.id.clone()),
        ..Default::default()
    };
    
    self.send_email_notification(
        &customer_email,
        EmailEventType::PaymentSucceeded,
        email_data,
    ).await?;
}
```

## Testing

### Test SMTP Configuration

```bash
# Set environment variables
export SMTP_HOST=smtp.sendgrid.net
export SMTP_PORT=587
export SMTP_USERNAME=apikey
export SMTP_PASSWORD=SG.your-test-api-key
export FROM_EMAIL=noreply@yourdomain.com
export FROM_NAME="NanoLambda Test"

# Start server
./target/release/nanolambda-server
```

### Trigger Test Emails

Use the webhook test script to trigger emails:

```bash
# This will trigger payment_succeeded email
./test-webhooks.sh
```

### Manual Test with curl

```bash
# Trigger a webhook that sends email
curl -X POST http://localhost:8080/webhooks/stripe \
  -H "Content-Type: application/json" \
  -H "Stripe-Signature: t=...,v1=..." \
  -d '{
    "type": "invoice.payment_succeeded",
    "data": {
      "object": {
        "id": "in_test123",
        "customer": "cus_test",
        "amount_paid": 1000
      }
    }
  }'
```

## Email Templates

### Customizing Templates

Email templates are defined in `payment.rs` in the `send_email_notification()` method. To customize:

1. Modify HTML template strings
2. Update colors, fonts, and styling
3. Add your logo (as embedded base64 or external URL)
4. Adjust copy and messaging

### Adding New Event Types

1. Add new variant to `EmailEventType` enum
2. Add case to `send_email_notification()` match statement
3. Define HTML and text templates
4. Populate `EmailEventData` with required fields
5. Call from appropriate webhook handler

## Monitoring

### Logging

All email operations are logged:

```rust
tracing::info!("Sent payment_succeeded email to user@example.com");
tracing::warn!("Failed to send email: Connection timeout");
```

Enable debug logging:
```bash
RUST_LOG=debug ./target/release/nanolambda-server
```

### Email Delivery Status

- **Success**: Logged as INFO
- **Failure**: Logged as WARN, webhook processing continues
- **Config Missing**: Logged as WARN, silently skipped

### Common Issues

**Issue**: Emails not sending  
**Check**:
- SMTP environment variables set correctly
- SMTP credentials valid
- Sender email verified with provider
- Firewall/network allows SMTP traffic (port 587/465)

**Issue**: Emails go to spam  
**Solutions**:
- Verify sender domain (SPF, DKIM, DMARC records)
- Use verified sending domain
- Avoid spam trigger words
- Include unsubscribe link
- Monitor sender reputation

**Issue**: "SMTP authentication failed"  
**Solutions**:
- Check username/password
- For SendGrid, username must be "apikey"
- For Gmail, use App Password not regular password
- Verify account not locked/suspended

## Best Practices

### Production Checklist

- [ ] Use dedicated SMTP service (SendGrid/SES, not Gmail)
- [ ] Verify sender domain with SPF/DKIM/DMARC
- [ ] Store real customer emails in database
- [ ] Implement email preferences (opt-out)
- [ ] Add unsubscribe link to all emails
- [ ] Monitor bounce/complaint rates
- [ ] Use separate email for transactional vs marketing
- [ ] Implement retry logic for failed sends
- [ ] Track email open/click rates
- [ ] Test emails across email clients

### Security

- Store SMTP credentials securely (environment variables, secrets manager)
- Use TLS for SMTP connection (port 587)
- Never log SMTP passwords
- Validate email addresses before sending
- Rate limit email sending
- Implement email verification for new accounts

### Compliance

- **CAN-SPAM Act**: Include unsubscribe link, physical address
- **GDPR**: Get consent, allow data deletion, privacy policy
- **Transactional Emails**: Generally exempt from opt-in requirements
- **Marketing Emails**: Require explicit consent

## Future Enhancements

- [ ] Email templates in separate files (not inline)
- [ ] Template engine (Handlebars, Tera)
- [ ] Email preferences management
- [ ] Unsubscribe links
- [ ] Email open/click tracking
- [ ] A/B testing for email copy
- [ ] Scheduled emails (trial reminders, usage alerts)
- [ ] Email queuing with retry logic
- [ ] Bounce/complaint handling
- [ ] Email preview in dashboard
- [ ] Multi-language support

## API Reference

### EmailEventType

```rust
pub enum EmailEventType {
    PaymentSucceeded,
    PaymentFailed,
    SubscriptionCreated,
    SubscriptionUpdated,
    SubscriptionCanceled,
    PaymentMethodAttached,
    TrialEnding,
}
```

### EmailEventData

```rust
pub struct EmailEventData {
    pub amount: Option<i64>,                    // Amount in cents
    pub invoice_id: Option<String>,
    pub subscription_id: Option<String>,
    pub tier: Option<String>,
    pub status: Option<String>,
    pub failure_reason: Option<String>,
    pub payment_method_last4: Option<String>,
    pub payment_method_brand: Option<String>,
    pub period_end: Option<i64>,                // Unix timestamp
    pub days_remaining: Option<i64>,
}
```

### EmailService

```rust
pub struct EmailService {
    smtp_host: String,
    smtp_port: u16,
    smtp_username: String,
    smtp_password: String,
    from_email: String,
    from_name: String,
}

impl EmailService {
    pub fn from_env() -> Result<Self>;
    pub async fn send_email(&self, to: &str, subject: &str, 
                           body_html: &str, body_text: &str) -> Result<()>;
}
```

## Resources

- [SendGrid SMTP Documentation](https://docs.sendgrid.com/for-developers/sending-email/integrating-with-the-smtp-api)
- [AWS SES SMTP Documentation](https://docs.aws.amazon.com/ses/latest/dg/send-email-smtp.html)
- [Lettre Crate Documentation](https://docs.rs/lettre/)
- [Email Template Best Practices](https://www.campaignmonitor.com/best-practice/)
- [CAN-SPAM Compliance](https://www.ftc.gov/business-guidance/resources/can-spam-act-compliance-guide-business)
