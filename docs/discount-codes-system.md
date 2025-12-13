# Discount Codes System Documentation

## Overview

The Discount Codes System enables promotional campaigns and revenue optimization through flexible coupon codes with percentage or fixed-amount discounts. The system includes usage tracking, expiration management, Stripe integration, and a user-friendly dashboard interface.

## Key Features

✅ **Flexible Discount Types**: Percentage-based or fixed-amount discounts  
✅ **Usage Limits**: Optional maximum redemption limits  
✅ **Expiration Dates**: Time-bound promotional campaigns  
✅ **Stripe Integration**: Automatic Stripe coupon creation and sync  
✅ **Usage Tracking**: Complete audit trail of discount applications  
✅ **Dashboard UI**: Interactive discount code validation and application  
✅ **Admin API**: Comprehensive discount management endpoints  
✅ **Case-Insensitive**: Codes work regardless of case (SAVE20 = save20)  
✅ **Validation**: Real-time discount calculation and eligibility checks  

## Database Schema

### discount_codes Table

```sql
CREATE TABLE IF NOT EXISTS discount_codes (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    code TEXT NOT NULL UNIQUE COLLATE NOCASE,  -- Case-insensitive unique constraint
    discount_type TEXT NOT NULL,                 -- "percentage" or "fixed"
    amount INTEGER NOT NULL,                     -- Percentage (1-100) or cents
    description TEXT NOT NULL,                   -- User-friendly description
    max_uses INTEGER,                            -- NULL = unlimited
    current_uses INTEGER NOT NULL DEFAULT 0,     -- Usage counter
    expires_at INTEGER,                          -- Unix timestamp, NULL = never expires
    stripe_coupon_id TEXT,                       -- Stripe coupon ID (optional)
    active INTEGER NOT NULL DEFAULT 1,           -- 1 = active, 0 = inactive
    created_at INTEGER NOT NULL,
    updated_at INTEGER NOT NULL
);

CREATE INDEX idx_discount_codes_code ON discount_codes(code);
CREATE INDEX idx_discount_codes_active ON discount_codes(active);
```

### discount_usage Table

```sql
CREATE TABLE IF NOT EXISTS discount_usage (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    discount_code_id INTEGER NOT NULL,
    api_key TEXT NOT NULL,                    -- User who applied the discount
    applied_amount INTEGER NOT NULL,          -- Actual discount in cents
    original_amount INTEGER NOT NULL,         -- Original price before discount
    final_amount INTEGER NOT NULL,            -- Final price after discount
    used_at INTEGER NOT NULL,                 -- Unix timestamp
    FOREIGN KEY (discount_code_id) REFERENCES discount_codes(id) ON DELETE CASCADE
);

CREATE INDEX idx_discount_usage_code_id ON discount_usage(discount_code_id);
CREATE INDEX idx_discount_usage_api_key ON discount_usage(api_key);
```

## Discount Types

### Percentage Discount

- **Type**: `percentage`
- **Amount Range**: 1-100 (represents percentage off)
- **Example**: `20` = 20% off
- **Calculation**: `discount = (original_amount * percentage) / 100`

### Fixed Amount Discount

- **Type**: `fixed`
- **Amount**: Cents (e.g., 1000 = $10.00)
- **Example**: `1000` = $10.00 off
- **Calculation**: `discount = min(fixed_amount, original_amount)`

## API Endpoints

### 1. Create Discount Code (Admin Only)

Creates a new discount code with optional Stripe coupon creation.

**Endpoint**: `POST /discounts`  
**Authentication**: Admin API Key (via `ADMIN_API_KEY` env var)  
**Content-Type**: `application/json`

**Request Body**:
```json
{
  "code": "SAVE20",
  "discount_type": "percentage",
  "amount": 20,
  "description": "20% off subscription",
  "max_uses": 100,
  "expires_at": 1735689600,
  "create_stripe_coupon": true
}
```

**Parameters**:
- `code` (required): Discount code string (will be uppercased)
- `discount_type` (required): "percentage" or "fixed"
- `amount` (required): Discount amount (1-100 for percentage, cents for fixed)
- `description` (required): User-friendly description
- `max_uses` (optional): Maximum number of redemptions (null = unlimited)
- `expires_at` (optional): Unix timestamp expiration (null = never expires)
- `create_stripe_coupon` (optional): Create corresponding Stripe coupon (default: false)

**Response** (200 OK):
```json
{
  "success": true,
  "discount": {
    "id": 1,
    "code": "SAVE20",
    "type": "percentage",
    "amount": 20,
    "description": "20% off subscription",
    "max_uses": 100,
    "current_uses": 0,
    "expires_at": 1735689600,
    "stripe_coupon_id": "SAVE20_abc123",
    "active": true,
    "created_at": 1703952000
  }
}
```

**Example**:
```bash
curl -X POST http://localhost:3000/discounts \
  -H "x-api-key: your-admin-key" \
  -H "Content-Type: application/json" \
  -d '{
    "code": "WELCOME25",
    "discount_type": "percentage",
    "amount": 25,
    "description": "25% off first month",
    "max_uses": 500,
    "expires_at": 1735689600,
    "create_stripe_coupon": true
  }'
```

---

### 2. Validate Discount Code (Public)

Validates a discount code and calculates the discount amount without applying it. No authentication required - useful for showing discount previews to users.

**Endpoint**: `POST /discounts/validate`  
**Authentication**: None (public endpoint)  
**Content-Type**: `application/json`

**Request Body**:
```json
{
  "code": "SAVE20",
  "amount": 5000
}
```

**Parameters**:
- `code` (required): Discount code to validate
- `amount` (required): Original amount in cents to test against

**Response** (200 OK - Valid Code):
```json
{
  "success": true,
  "valid": true,
  "error_message": null,
  "calculated_discount": 1000,
  "discount_code": {
    "id": 1,
    "code": "SAVE20",
    "type": "percentage",
    "amount": 20,
    "description": "20% off subscription"
  }
}
```

**Response** (200 OK - Invalid Code):
```json
{
  "success": true,
  "valid": false,
  "error_message": "Discount code has expired",
  "calculated_discount": 0,
  "discount_code": {
    "id": 1,
    "code": "SAVE20",
    "type": "percentage",
    "amount": 20,
    "description": "20% off subscription"
  }
}
```

**Validation Checks**:
1. Code exists in database
2. Code is active
3. Code hasn't expired (if expiration set)
4. Usage limit not exceeded (if limit set)

**Example**:
```bash
curl -X POST http://localhost:3000/discounts/validate \
  -H "Content-Type: application/json" \
  -d '{
    "code": "SAVE20",
    "amount": 5000
  }'
```

---

### 3. Apply Discount Code (Authenticated)

Applies a discount code to a purchase, recording the usage and incrementing the counter.

**Endpoint**: `POST /discounts/apply`  
**Authentication**: Required (API Key)  
**Content-Type**: `application/json`

**Request Body**:
```json
{
  "code": "SAVE20",
  "amount": 5000
}
```

**Parameters**:
- `code` (required): Discount code to apply
- `amount` (required): Original amount in cents

**Response** (200 OK):
```json
{
  "success": true,
  "usage": {
    "id": 42,
    "discount_code_id": 1,
    "applied_amount": 1000,
    "original_amount": 5000,
    "final_amount": 4000,
    "used_at": 1703952000
  }
}
```

**Response** (400 Bad Request - Invalid Code):
```json
{
  "error": "DiscountApplicationFailed",
  "message": "Discount code has reached maximum usage"
}
```

**Example**:
```bash
curl -X POST http://localhost:3000/discounts/apply \
  -H "x-api-key: your-api-key" \
  -H "Content-Type: application/json" \
  -d '{
    "code": "SAVE20",
    "amount": 5000
  }'
```

---

### 4. List Discount Codes (Admin Only)

Lists all discount codes with optional filtering by active status.

**Endpoint**: `GET /discounts?active_only=true`  
**Authentication**: Admin API Key  
**Query Parameters**:
- `active_only` (optional): Filter for active codes only (default: false)

**Response** (200 OK):
```json
{
  "success": true,
  "discounts": [
    {
      "id": 1,
      "code": "SAVE20",
      "type": "percentage",
      "amount": 20,
      "description": "20% off subscription",
      "max_uses": 100,
      "current_uses": 42,
      "expires_at": 1735689600,
      "stripe_coupon_id": "SAVE20_abc123",
      "active": true,
      "created_at": 1703952000
    }
  ],
  "count": 1
}
```

**Example**:
```bash
curl http://localhost:3000/discounts?active_only=true \
  -H "x-api-key: your-admin-key"
```

---

### 5. Get Discount Usage History (Admin Only)

Retrieves usage history for a specific discount code.

**Endpoint**: `GET /discounts/:discount_id/usage`  
**Authentication**: Admin API Key  
**Path Parameters**:
- `discount_id`: Discount code ID (integer)

**Response** (200 OK):
```json
{
  "success": true,
  "usage": [
    {
      "id": 1,
      "discount_code_id": 1,
      "api_key": "user-123-api-key",
      "applied_amount": 1000,
      "original_amount": 5000,
      "final_amount": 4000,
      "used_at": 1703952000
    }
  ],
  "count": 1
}
```

**Example**:
```bash
curl http://localhost:3000/discounts/1/usage \
  -H "x-api-key: your-admin-key"
```

---

### 6. Get User's Discount Usage (Authenticated)

Retrieves the authenticated user's own discount usage history.

**Endpoint**: `GET /discounts/my-usage`  
**Authentication**: Required (API Key)

**Response** (200 OK):
```json
{
  "success": true,
  "usage": [
    {
      "id": 1,
      "discount_code_id": 1,
      "applied_amount": 1000,
      "original_amount": 5000,
      "final_amount": 4000,
      "used_at": 1703952000
    }
  ],
  "count": 1
}
```

**Example**:
```bash
curl http://localhost:3000/discounts/my-usage \
  -H "x-api-key: your-api-key"
```

## Programmatic Usage (Rust)

### Creating a Discount Code

```rust
use nanolambda_storage::discount::{DiscountManager, DiscountType};
use sqlx::SqlitePool;
use chrono::Utc;

// Initialize discount manager
let pool = SqlitePool::connect("sqlite://nanolambda.db.usage.db").await?;
let stripe_key = std::env::var("STRIPE_SECRET_KEY")?;
let manager = DiscountManager::new(stripe_key, pool).await?;

// Create percentage discount
let discount = manager.create_discount(
    "SAVE20".to_string(),
    DiscountType::Percentage,
    20, // 20%
    "20% off subscription".to_string(),
    Some(100), // Max 100 uses
    Some(Utc::now().timestamp() + (30 * 24 * 60 * 60)), // Expires in 30 days
    None, // No Stripe coupon yet
).await?;

println!("Created discount: {} (ID: {})", discount.code, discount.id);

// Optionally create Stripe coupon
if let Some(coupon_id) = manager.create_stripe_coupon(discount.id).await.ok() {
    println!("Stripe coupon created: {}", coupon_id);
}
```

### Validating a Discount Code

```rust
// Validate without applying
let validation = manager.validate_discount("SAVE20", 5000).await?;

if validation.valid {
    let discount = validation.discount_code.unwrap();
    println!(
        "Valid! Discount: ${:.2} ({}% off)",
        validation.calculated_discount as f64 / 100.0,
        (validation.calculated_discount * 100) / 5000
    );
} else {
    eprintln!("Invalid: {}", validation.error_message.unwrap());
}
```

### Applying a Discount Code

```rust
// Apply and record usage
let usage = manager.apply_discount(
    "SAVE20",
    "user-api-key-123",
    5000 // $50.00 in cents
).await?;

println!(
    "Applied discount! Original: ${:.2}, Discount: ${:.2}, Final: ${:.2}",
    usage.original_amount as f64 / 100.0,
    usage.applied_amount as f64 / 100.0,
    usage.final_amount as f64 / 100.0
);
```

### Listing Discount Codes

```rust
// Get all active discount codes
let discounts = manager.list_discounts(true).await?;

for discount in discounts {
    println!(
        "{}: {} ({} type, {} uses)",
        discount.code,
        discount.description,
        discount.discount_type.as_str(),
        discount.current_uses
    );
}
```

### Getting Usage Statistics

```rust
// Get usage history for a discount code
let usage = manager.get_discount_usage(discount_id).await?;

let total_savings: i64 = usage.iter()
    .map(|u| u.applied_amount)
    .sum();

println!(
    "Code used {} times, total savings: ${:.2}",
    usage.len(),
    total_savings as f64 / 100.0
);

// Get a user's discount usage
let user_usage = manager.get_user_discount_usage("user-api-key").await?;
println!("User has used {} discounts", user_usage.len());
```

### Deactivating a Discount Code

```rust
// Deactivate (soft delete)
manager.deactivate_discount("SAVE20").await?;
println!("Discount code deactivated");

// Reactivate
manager.reactivate_discount("SAVE20").await?;
println!("Discount code reactivated");
```

## Dashboard Integration

### Interactive Discount Code Dialog

The dashboard includes a user-friendly discount code interface accessible via the "💸 Apply Discount Code" button in the billing section.

**Features**:
- Real-time validation as user types
- Visual feedback (green for valid, red for invalid)
- Discount calculation preview
- One-click application
- Responsive modal design

**JavaScript Functions**:

```javascript
// Show discount code dialog
showDiscountDialog();

// Validate code and show preview
async function validateDiscountCode(code, amount) {
    const response = await fetch('/discounts/validate', {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ code, amount })
    });
    return await response.json();
}

// Apply discount code
async function applyDiscountCode(code, amount) {
    const response = await fetch('/discounts/apply', {
        method: 'POST',
        headers: {
            'Content-Type': 'application/json',
            'Authorization': `Bearer ${apiKey}`
        },
        body: JSON.stringify({ code, amount })
    });
    return await response.json();
}
```

**User Flow**:
1. User clicks "💸 Apply Discount Code" button
2. Modal opens with input field
3. User enters code (e.g., "SAVE20")
4. Clicks "Validate Code" or presses Enter
5. System validates and shows discount preview
6. User clicks "Apply to Next Payment"
7. Discount is applied and confirmation shown

## Stripe Integration

### Creating Stripe Coupons

When creating a discount code, you can optionally create a corresponding Stripe coupon:

```rust
// Create discount with Stripe coupon
let discount = manager.create_discount(
    "SAVE20".to_string(),
    DiscountType::Percentage,
    20,
    "20% off".to_string(),
    None,
    None,
    None
).await?;

// Create Stripe coupon
let coupon_id = manager.create_stripe_coupon(discount.id).await?;
println!("Stripe coupon ID: {}", coupon_id);
```

**Stripe Coupon Mapping**:
- **Percentage Discounts**: `percent_off` parameter
- **Fixed Discounts**: `amount_off` + `currency` parameters
- **Max Redemptions**: `max_redemptions` parameter
- **Expiration**: `redeem_by` parameter

### Applying Stripe Coupons to Subscriptions

```bash
# Create subscription with discount
curl -X POST http://localhost:3000/payment/subscription \
  -H "x-api-key: your-api-key" \
  -H "Content-Type: application/json" \
  -d '{
    "price_id": "price_123",
    "coupon": "SAVE20_abc123"
  }'
```

## Configuration

### Environment Variables

```bash
# Admin API key for creating/managing discounts
export ADMIN_API_KEY="admin-your-secure-key-here"

# Stripe API key for coupon creation
export STRIPE_SECRET_KEY="sk_live_..."
```

### Security Considerations

1. **Admin Endpoints**: Protected by `ADMIN_API_KEY` environment variable
2. **User Endpoints**: Protected by standard API key authentication
3. **Public Validation**: No authentication required (safe for preview purposes)
4. **Case-Insensitive Codes**: Prevents duplicate code issues
5. **SQL Injection**: Protected via parameterized queries
6. **Rate Limiting**: Standard rate limits apply to all endpoints

## Common Use Cases

### 1. Launch Promotion

```bash
# Create 30% off code for first 1000 users, expires in 7 days
curl -X POST http://localhost:3000/discounts \
  -H "x-api-key: your-admin-key" \
  -H "Content-Type: application/json" \
  -d '{
    "code": "LAUNCH30",
    "discount_type": "percentage",
    "amount": 30,
    "description": "Launch special: 30% off",
    "max_uses": 1000,
    "expires_at": 1704556800,
    "create_stripe_coupon": true
  }'
```

### 2. Referral Discount

```bash
# Create $10 off code for referrals (unlimited uses, no expiration)
curl -X POST http://localhost:3000/discounts \
  -H "x-api-key: your-admin-key" \
  -H "Content-Type: application/json" \
  -d '{
    "code": "FRIEND10",
    "discount_type": "fixed",
    "amount": 1000,
    "description": "$10 off from friend referral",
    "create_stripe_coupon": true
  }'
```

### 3. Seasonal Campaign

```bash
# Create holiday discount (50% off, 500 uses, 14-day duration)
curl -X POST http://localhost:3000/discounts \
  -H "x-api-key: your-admin-key" \
  -H "Content-Type: application/json" \
  -d '{
    "code": "HOLIDAY50",
    "discount_type": "percentage",
    "amount": 50,
    "description": "Holiday special: Half off!",
    "max_uses": 500,
    "expires_at": 1704556800,
    "create_stripe_coupon": true
  }'
```

### 4. VIP Customer Code

```bash
# Create exclusive code for specific customer (single use)
curl -X POST http://localhost:3000/discounts \
  -H "x-api-key: your-admin-key" \
  -H "Content-Type: application/json" \
  -d '{
    "code": "VIP-CUSTOMER-123",
    "discount_type": "percentage",
    "amount": 100,
    "description": "VIP: Free first month",
    "max_uses": 1,
    "create_stripe_coupon": true
  }'
```

## Error Handling

### Common Errors

**Code Not Found**:
```json
{
  "valid": false,
  "error_message": "Discount code not found"
}
```

**Code Expired**:
```json
{
  "valid": false,
  "error_message": "Discount code has expired"
}
```

**Usage Limit Reached**:
```json
{
  "valid": false,
  "error_message": "Discount code has reached maximum usage"
}
```

**Code Inactive**:
```json
{
  "valid": false,
  "error_message": "Discount code is no longer active"
}
```

**Invalid Percentage**:
```json
{
  "error": "InvalidConfig",
  "message": "Percentage discount must be between 1 and 100"
}
```

**Unauthorized (Admin Endpoints)**:
```json
{
  "error": "Forbidden",
  "message": "Admin privileges required"
}
```

## Best Practices

### 1. Code Naming Conventions

- **Promotional**: `SAVE20`, `WELCOME25`, `SPRING30`
- **Seasonal**: `HOLIDAY50`, `NEWYEAR25`, `BLACKFRIDAY`
- **Referral**: `FRIEND10`, `REFER25`
- **VIP**: `VIP-CUSTOMER-{ID}`, `EXCLUSIVE-{CODE}`
- **Partner**: `PARTNER-{NAME}-{DISCOUNT}`

### 2. Discount Amounts

- **Percentage**: Start with 10-25% for general promotions
- **Fixed**: $5-$20 for subscription-based products
- **Limited-Time**: 30-50% for flash sales
- **Referral**: $10 or 20% standard

### 3. Usage Limits

- **Launch**: 500-1000 uses
- **Flash Sale**: 100-500 uses
- **General Promo**: 5000+ uses or unlimited
- **Referral**: Unlimited
- **VIP/Custom**: 1 use per code

### 4. Expiration Strategy

- **Short-Term**: 3-7 days (urgency)
- **Medium-Term**: 30 days (standard promo)
- **Long-Term**: 90+ days (partner codes)
- **Referral**: No expiration
- **Seasonal**: Align with holiday/event

### 5. Monitoring and Analytics

```rust
// Track discount performance
let usage = manager.get_discount_usage(discount_id).await?;

let stats = DiscountStats {
    total_uses: usage.len(),
    total_revenue_impact: usage.iter().map(|u| u.applied_amount).sum(),
    average_discount: usage.iter().map(|u| u.applied_amount).sum::<i64>() / usage.len() as i64,
    conversion_rate: (usage.len() as f64 / total_validations as f64) * 100.0,
};

println!("Discount Stats: {:#?}", stats);
```

### 6. A/B Testing

Create multiple variants to test effectiveness:

```bash
# Variant A: 20% off
curl -X POST ... -d '{"code": "TEST-A", "amount": 20, ...}'

# Variant B: $10 off
curl -X POST ... -d '{"code": "TEST-B", "amount": 1000, ...}'

# Compare usage rates and revenue impact
```

## Testing

### Unit Tests

The discount module includes comprehensive unit tests:

```bash
cargo test --package nanolambda-storage discount
```

**Test Coverage**:
- ✅ Percentage discount calculation
- ✅ Fixed discount calculation
- ✅ Expiration validation
- ✅ Usage limit enforcement
- ✅ Case-insensitive code matching
- ✅ Active/inactive status checks

### Manual Testing

```bash
# 1. Create test discount
curl -X POST http://localhost:3000/discounts \
  -H "x-api-key: admin-key-change-me" \
  -H "Content-Type: application/json" \
  -d '{
    "code": "TEST20",
    "discount_type": "percentage",
    "amount": 20,
    "description": "Test discount",
    "max_uses": 5
  }'

# 2. Validate code
curl -X POST http://localhost:3000/discounts/validate \
  -H "Content-Type: application/json" \
  -d '{"code": "TEST20", "amount": 5000}'

# 3. Apply code
curl -X POST http://localhost:3000/discounts/apply \
  -H "x-api-key: your-api-key" \
  -H "Content-Type: application/json" \
  -d '{"code": "TEST20", "amount": 5000}'

# 4. Check usage
curl http://localhost:3000/discounts/1/usage \
  -H "x-api-key: admin-key-change-me"
```

## Troubleshooting

### Issue: "Discount code not found"

**Cause**: Code doesn't exist or was typed incorrectly  
**Solution**: 
- Check code spelling (case doesn't matter)
- Verify code exists: `GET /discounts`
- Create code if needed

### Issue: "Discount code has expired"

**Cause**: Current timestamp exceeds `expires_at`  
**Solution**:
- Check expiration: `SELECT expires_at FROM discount_codes WHERE code = 'X'`
- Extend expiration or create new code

### Issue: "Maximum usage reached"

**Cause**: `current_uses >= max_uses`  
**Solution**:
- Increase `max_uses` limit
- Create new code with fresh limit
- Check usage: `GET /discounts/:id/usage`

### Issue: "Admin privileges required"

**Cause**: Wrong API key or missing `ADMIN_API_KEY` env var  
**Solution**:
- Set `ADMIN_API_KEY` environment variable
- Use correct admin key in `x-api-key` header

### Issue: Stripe coupon creation fails

**Cause**: Invalid Stripe API key or parameters  
**Solution**:
- Verify `STRIPE_SECRET_KEY` is set correctly
- Check Stripe dashboard for errors
- Ensure discount parameters are valid for Stripe

## Future Enhancements

1. **Auto-Apply Codes**: Automatically apply best available discount
2. **Stacking Rules**: Allow/disallow multiple discount combinations
3. **User-Specific Codes**: Generate unique codes per user
4. **Tiered Discounts**: Different amounts based on purchase size
5. **Referral Integration**: Auto-generate codes for referral program
6. **Campaign Analytics**: Advanced reporting and insights
7. **Scheduled Activation**: Auto-activate codes at specific time
8. **Geographic Restrictions**: Limit codes by region
9. **Product-Specific**: Apply only to certain tiers/features
10. **Bulk Code Generation**: Create 1000s of unique codes at once

## Support

For questions or issues with the discount codes system:
1. Check this documentation first
2. Review error messages carefully
3. Test with simple scenarios
4. Check database state directly if needed
5. Contact support with specific error details

---

**Version**: 1.0  
**Last Updated**: 2024  
**Component**: Discount Codes System (Task #14)
