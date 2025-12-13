# Invoice Generation and Storage System

## Overview

The NanoLambda invoice system provides automated invoice generation, storage, and management capabilities with full Stripe integration. This system enables transparent billing, easy access to payment history, and self-service invoice management.

## Key Features

- **Automated Invoice Creation**: Generate invoices programmatically for subscriptions and one-time charges
- **Stripe Integration**: Full synchronization with Stripe invoices including PDFs and hosted URLs
- **Invoice Storage**: SQLite-based persistent storage for all invoices and line items
- **RESTful API**: Complete API for invoice operations (list, get, summary, sync)
- **Dashboard UI**: Interactive invoice viewer with detailed breakdowns
- **Multi-Status Support**: Draft, Open, Paid, Void, and Uncollectible status tracking
- **Line Item Support**: Detailed breakdown of charges per invoice

## Database Schema

### Invoices Table

```sql
CREATE TABLE invoices (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    api_key TEXT NOT NULL,
    stripe_invoice_id TEXT UNIQUE,
    invoice_number TEXT NOT NULL UNIQUE,
    amount_due INTEGER NOT NULL,          -- in cents
    amount_paid INTEGER NOT NULL DEFAULT 0,
    currency TEXT NOT NULL DEFAULT 'usd',
    status TEXT NOT NULL,                 -- draft, open, paid, void, uncollectible
    period_start INTEGER NOT NULL,        -- Unix timestamp
    period_end INTEGER NOT NULL,
    due_date INTEGER,
    paid_at INTEGER,
    tier_level TEXT NOT NULL,
    description TEXT NOT NULL,
    invoice_pdf_url TEXT,                 -- Stripe PDF URL
    hosted_invoice_url TEXT,              -- Stripe hosted invoice URL
    created_at INTEGER NOT NULL,
    updated_at INTEGER NOT NULL,
    FOREIGN KEY (api_key) REFERENCES api_keys(key)
);

-- Indexes for efficient queries
CREATE INDEX idx_invoices_api_key ON invoices(api_key);
CREATE INDEX idx_invoices_stripe_id ON invoices(stripe_invoice_id);
CREATE INDEX idx_invoices_status ON invoices(status);
```

### Invoice Line Items Table

```sql
CREATE TABLE invoice_line_items (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    invoice_id INTEGER NOT NULL,
    description TEXT NOT NULL,
    quantity INTEGER NOT NULL,
    unit_amount INTEGER NOT NULL,        -- in cents
    amount INTEGER NOT NULL,              -- quantity * unit_amount
    currency TEXT NOT NULL DEFAULT 'usd',
    period_start INTEGER NOT NULL,
    period_end INTEGER NOT NULL,
    FOREIGN KEY (invoice_id) REFERENCES invoices(id) ON DELETE CASCADE
);

CREATE INDEX idx_invoice_line_items_invoice_id ON invoice_line_items(invoice_id);
```

## Invoice Number Format

Invoices are automatically assigned unique numbers in the format:

```
INV-YYYYMM-XXXX
```

- `YYYYMM`: Year and month of creation (e.g., 202412)
- `XXXX`: Random 4-digit number (0000-9999)

Example: `INV-202412-4823`

## Invoice Statuses

| Status | Description |
|--------|-------------|
| `draft` | Invoice created but not finalized |
| `open` | Finalized invoice awaiting payment |
| `paid` | Invoice fully paid |
| `void` | Invoice voided/cancelled |
| `uncollectible` | Payment failed and marked uncollectible |

## API Endpoints

### 1. List Invoices

Get all invoices for the authenticated user.

**Endpoint**: `GET /invoices`

**Headers**:
```
Authorization: Bearer {api_key}
```

**Response**:
```json
{
  "success": true,
  "invoices": [
    {
      "id": 1,
      "api_key": "nl_...",
      "stripe_invoice_id": "in_...",
      "invoice_number": "INV-202412-4823",
      "amount_due": 5000,
      "amount_paid": 5000,
      "currency": "usd",
      "status": "paid",
      "period_start": 1701388800,
      "period_end": 1704067200,
      "due_date": null,
      "paid_at": 1702598400,
      "tier_level": "pro",
      "description": "Pro Subscription - December 2023",
      "invoice_pdf_url": "https://pay.stripe.com/invoice/.../pdf",
      "hosted_invoice_url": "https://invoice.stripe.com/i/...",
      "created_at": 1701388800,
      "updated_at": 1702598400
    }
  ],
  "count": 1
}
```

### 2. Get Invoice by ID

Retrieve detailed invoice information including line items.

**Endpoint**: `GET /invoices/{invoice_id}`

**Headers**:
```
Authorization: Bearer {api_key}
```

**Response**:
```json
{
  "success": true,
  "invoice": {
    "id": 1,
    "invoice_number": "INV-202412-4823",
    "amount_due": 5000,
    "status": "paid",
    ...
  },
  "line_items": [
    {
      "id": 1,
      "invoice_id": 1,
      "description": "Pro Plan - Monthly Subscription",
      "quantity": 1,
      "unit_amount": 5000,
      "amount": 5000,
      "currency": "usd",
      "period_start": 1701388800,
      "period_end": 1704067200
    }
  ]
}
```

### 3. Get Invoice Summary

Get summary statistics for all invoices.

**Endpoint**: `GET /invoices/summary`

**Headers**:
```
Authorization: Bearer {api_key}
```

**Response**:
```json
{
  "success": true,
  "summary": {
    "total_invoices": 12,
    "total_paid": 60000,
    "total_outstanding": 5000,
    "recent_invoices": [
      {
        "id": 12,
        "invoice_number": "INV-202412-4823",
        "amount_due": 5000,
        "status": "open",
        ...
      }
    ]
  }
}
```

### 4. Sync Invoice from Stripe

Refresh invoice data from Stripe (status, PDFs, URLs).

**Endpoint**: `POST /invoices/sync/{stripe_invoice_id}`

**Headers**:
```
Authorization: Bearer {api_key}
```

**Response**:
```json
{
  "success": true,
  "invoice": {
    "id": 1,
    "stripe_invoice_id": "in_...",
    "status": "paid",
    "invoice_pdf_url": "https://pay.stripe.com/invoice/.../pdf",
    "hosted_invoice_url": "https://invoice.stripe.com/i/...",
    ...
  },
  "message": "Invoice synced successfully"
}
```

## Programmatic Usage

### Creating an Invoice (Rust)

```rust
use nanolambda_storage::invoice::{InvoiceManager, Invoice};
use nanolambda_storage::tier::TierLevel;
use chrono::Utc;

// Initialize manager
let invoice_mgr = InvoiceManager::new(stripe_api_key, pool).await?;

// Create invoice
let now = Utc::now().timestamp();
let period_end = now + (30 * 24 * 60 * 60); // 30 days

let invoice = invoice_mgr.create_invoice(
    "nl_api_key_123",
    TierLevel::Pro,
    5000, // $50.00 in cents
    now,
    period_end,
    "Pro Subscription - December 2023".to_string(),
    Some("in_stripe_id_123".to_string())
).await?;

// Add line item
let line_item = invoice_mgr.add_line_item(
    invoice.id,
    "Pro Plan - Monthly Subscription".to_string(),
    1,
    5000,
    now,
    period_end
).await?;
```

### Listing Invoices

```rust
// Get all invoices
let invoices = invoice_mgr.list_invoices(
    "nl_api_key_123",
    Some(100), // limit
    None // no status filter
).await?;

// Get only paid invoices
use nanolambda_storage::invoice::InvoiceStatus;
let paid_invoices = invoice_mgr.list_invoices(
    "nl_api_key_123",
    Some(50),
    Some(InvoiceStatus::Paid)
).await?;
```

### Updating Invoice Status

```rust
use nanolambda_storage::invoice::InvoiceStatus;

// Mark as paid
invoice_mgr.update_invoice_status(
    invoice_id,
    InvoiceStatus::Paid,
    Some(5000), // amount paid
    Some(Utc::now().timestamp()) // paid at
).await?;

// Void invoice
invoice_mgr.update_invoice_status(
    invoice_id,
    InvoiceStatus::Void,
    None,
    None
).await?;
```

### Syncing from Stripe

```rust
// Sync invoice data from Stripe
let synced_invoice = invoice_mgr.sync_stripe_invoice("in_stripe_id_123").await?;

// This updates:
// - Status (draft, open, paid, void, uncollectible)
// - Amount paid
// - Paid timestamp
// - PDF URL
// - Hosted invoice URL
```

## Dashboard Integration

The invoice system includes a complete dashboard UI with:

### Features

1. **Invoice Summary Cards**:
   - Total invoices count
   - Total amount paid
   - Outstanding balance

2. **Recent Invoices List**:
   - Invoice number with clickable rows
   - Status badges (color-coded)
   - Amount due
   - Creation date

3. **Invoice Detail Modal**:
   - Full invoice information
   - Line items breakdown
   - Status and dates
   - Download PDF button (if available)
   - View online button (if available)

### JavaScript API

```javascript
// Fetch invoices
const invoices = await fetchInvoices();

// Fetch summary
const summary = await fetchInvoiceSummary();

// View invoice details
await viewInvoiceDetails(invoiceId);

// Render summary in dashboard
const html = renderInvoicesSummary(summary);
```

## Integration with Webhooks

Invoice status updates can be automated via Stripe webhooks:

```rust
// In webhook handler
match event_type {
    "invoice.paid" => {
        let stripe_invoice_id = /* extract from webhook */;
        let invoice = invoice_mgr.sync_stripe_invoice(&stripe_invoice_id).await?;
        // Invoice status and paid_at automatically updated
    },
    "invoice.payment_failed" => {
        let stripe_invoice_id = /* extract from webhook */;
        let invoice = invoice_mgr.sync_stripe_invoice(&stripe_invoice_id).await?;
        // Invoice status updated, email notification sent
    },
    _ => {}
}
```

## Error Handling

### Common Errors

| Error | Status Code | Description |
|-------|-------------|-------------|
| `Unauthorized` | 401 | Missing or invalid API key |
| `Forbidden` | 403 | Invoice doesn't belong to this user |
| `NotFound` | 404 | Invoice not found |
| `DatabaseError` | 500 | Database operation failed |
| `ExternalApiError` | 500 | Stripe API error |

### Example Error Response

```json
{
  "error": "NotFound",
  "message": "Invoice not found"
}
```

## Best Practices

### 1. Invoice Creation

- Always create invoices for subscription renewals
- Include descriptive line items
- Set appropriate period_start and period_end
- Link to Stripe invoices when available

### 2. Status Management

- Sync with Stripe regularly to keep status current
- Use webhooks for real-time updates
- Handle payment failures gracefully

### 3. Data Retention

- Keep invoices for accounting purposes (7+ years recommended)
- Don't delete paid invoices
- Use 'void' status instead of deletion

### 4. Customer Communication

- Send email notifications on invoice creation
- Provide PDF downloads for record-keeping
- Include invoice numbers in all communications

### 5. Security

- Always verify invoice ownership before displaying
- Use HTTPS for PDF and hosted URLs
- Validate Stripe webhook signatures

## Testing

### Manual Testing

1. **Create Test Invoice**:
```bash
# Via curl (requires authentication)
curl -X POST http://localhost:3000/invoices \
  -H "Authorization: Bearer nl_test_key" \
  -H "Content-Type: application/json" \
  -d '{
    "amount_due": 5000,
    "description": "Test Invoice",
    "period_start": 1701388800,
    "period_end": 1704067200
  }'
```

2. **List Invoices**:
```bash
curl -X GET http://localhost:3000/invoices \
  -H "Authorization: Bearer nl_test_key"
```

3. **View in Dashboard**:
- Navigate to dashboard
- Enter your API key
- Scroll to "Invoices Overview" section
- Click any invoice to view details

### Automated Testing

```rust
#[tokio::test]
async fn test_invoice_creation() {
    let pool = /* create test pool */;
    let invoice_mgr = InvoiceManager::new("sk_test_key".to_string(), pool).await.unwrap();
    
    let invoice = invoice_mgr.create_invoice(
        "test_key",
        TierLevel::Pro,
        5000,
        1701388800,
        1704067200,
        "Test Invoice".to_string(),
        None
    ).await.unwrap();
    
    assert_eq!(invoice.amount_due, 5000);
    assert_eq!(invoice.status, InvoiceStatus::Open);
}
```

## Troubleshooting

### Invoice not showing in dashboard

- Check API key is correct
- Verify invoice belongs to the authenticated user
- Check browser console for API errors
- Ensure `/invoices/summary` endpoint returns data

### Stripe sync failing

- Verify `STRIPE_SECRET_KEY` is set correctly
- Check Stripe invoice ID is valid
- Ensure invoice exists in Stripe
- Review Stripe API error messages

### Missing PDF URLs

- PDFs are only available after Stripe invoice is finalized
- Call `sync_stripe_invoice()` to fetch latest URLs
- Check Stripe dashboard for PDF generation status

### Outstanding balance incorrect

- Ensure `amount_paid` is updated correctly
- Sync with Stripe for accurate payment status
- Check for partially paid invoices

## Future Enhancements

1. **PDF Generation**: Generate PDFs locally without Stripe dependency
2. **Email Delivery**: Automated invoice email delivery
3. **Payment Reminders**: Scheduled reminders for unpaid invoices
4. **Bulk Operations**: Batch invoice creation and updates
5. **Invoice Templates**: Customizable invoice layouts
6. **Tax Support**: Automatic tax calculation and display
7. **Multi-Currency**: Support for currencies beyond USD
8. **Export**: CSV/Excel export for accounting systems
9. **Recurring Invoices**: Automated recurring billing
10. **Payment Plans**: Support for installment payments

## Related Documentation

- [Tier System](./tier-system.md) - Subscription tier management
- [Usage Tracking](./usage-tracking.md) - Usage-based billing
- [Stripe Integration](./stripe-integration.md) - Payment processing
- [Webhook Handlers](./webhooks.md) - Real-time event processing
- [Customer Portal](./customer-portal.md) - Self-service billing management

## Support

For issues or questions about the invoice system:
- Check the troubleshooting section above
- Review API endpoint documentation
- Inspect browser console for errors
- Contact support with invoice number and error details
