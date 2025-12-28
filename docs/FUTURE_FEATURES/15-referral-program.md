# Task #15: Referral Program System

**Status**: ✅ Complete  
**Version**: 0.1.0  
**Last Updated**: 2024

## Overview

The Referral Program System is a viral growth mechanism that incentivizes existing customers to bring in new users. It tracks referral clicks, activates referrals when new customers sign up, and rewards referrers with discounts or credits.

## Features

### Core Functionality

- **Unique Referral Codes**: Each user gets a unique referral code that tracks their referred customers
- **Referral Tracking**: Track clicks from referral links with UTM parameters and geographic data
- **Reward Management**: Support for percentage-based and fixed-amount rewards
- **Referral Leaderboard**: Public leaderboard showing top referrers
- **Dashboard Integration**: Manage referrals directly from the user dashboard
- **Social Sharing**: Share referral links via email, Twitter, LinkedIn, or copy link

### Reward Types

- **Percentage Rewards**: X% discount on next month's subscription (e.g., 10% = $5 off $50)
- **Fixed Rewards**: Fixed dollar amount credit (e.g., $10 off)
- **Maximum Referrals**: Optional cap on number of successful referrals per code

## Database Schema

### referral_codes Table

Stores referral codes and their configuration:

```sql
CREATE TABLE referral_codes (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    referrer_api_key TEXT NOT NULL UNIQUE,
    code TEXT NOT NULL UNIQUE,
    display_name TEXT NOT NULL,
    reward_type TEXT NOT NULL,  -- "percentage" or "fixed"
    reward_amount INTEGER NOT NULL,  -- in cents for fixed, percent for percentage
    reward_description TEXT NOT NULL,
    max_referrals INTEGER,  -- NULL = unlimited
    current_referrals INTEGER DEFAULT 0,
    active BOOLEAN DEFAULT 1,
    created_at INTEGER NOT NULL,
    updated_at INTEGER NOT NULL
);

CREATE INDEX idx_referrer_api_key ON referral_codes(referrer_api_key);
CREATE INDEX idx_code ON referral_codes(code);
CREATE INDEX idx_active ON referral_codes(active);
```

### referral_rewards Table

Tracks referral conversions and reward distribution:

```sql
CREATE TABLE referral_rewards (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    referral_code_id INTEGER NOT NULL,
    referrer_api_key TEXT NOT NULL,
    referred_api_key TEXT,
    referred_email TEXT NOT NULL,
    status TEXT DEFAULT 'pending',  -- pending, activated, claimed
    reward_earned BOOLEAN DEFAULT 0,
    reward_amount INTEGER,  -- in cents
    discount_code_id TEXT,
    tracking_data TEXT,  -- JSON with utm_source, utm_campaign, etc.
    referred_at INTEGER NOT NULL,
    activated_at INTEGER,
    claimed_at INTEGER,
    created_at INTEGER NOT NULL,
    updated_at INTEGER NOT NULL,
    
    FOREIGN KEY (referral_code_id) REFERENCES referral_codes(id) ON DELETE CASCADE
);

CREATE INDEX idx_referral_code_id ON referral_rewards(referral_code_id);
CREATE INDEX idx_referrer_api_key ON referral_rewards(referrer_api_key);
CREATE INDEX idx_status ON referral_rewards(status);
```

## API Endpoints

### Authentication

All protected endpoints require the `x-api-key` header with the user's API key.

### Protected Endpoints

#### Generate Referral Code
```
POST /referrals/generate
Authorization: x-api-key <api_key>

Request Body:
{
    "display_name": "My Referral",
    "reward_type": "percentage",
    "reward_amount": 10,
    "max_referrals": null
}

Response:
{
    "success": true,
    "referral": {
        "id": 1,
        "code": "ref-john-abc123",
        "display_name": "My Referral",
        "reward_type": "percentage",
        "reward_amount": 10,
        "reward_description": "10% off",
        "active": true,
        "created_at": 1234567890
    }
}

Errors:
- 400: Invalid reward type (must be "percentage" or "fixed")
- 400: Reward amount must be positive
- 409: User already has a referral code
```

#### Get My Referral Code
```
GET /referrals/my-code
Authorization: x-api-key <api_key>

Response:
{
    "success": true,
    "referral": {
        "code": "ref-john-abc123",
        "display_name": "My Referral",
        "reward_description": "10% off",
        "active": true,
        "created_at": 1234567890
    },
    "stats": {
        "total_referrals": 5,
        "active_referrals": 3,
        "pending_referrals": 2,
        "total_rewards_earned": 1500,  // in cents
        "total_rewards_value": "$15.00"
    }
}

Errors:
- 404: No referral code found for user
```

#### Get My Referral Rewards
```
GET /referrals/my-rewards
Authorization: x-api-key <api_key>

Response:
{
    "success": true,
    "rewards": [
        {
            "id": 1,
            "referred_email": "user@example.com",
            "status": "activated",
            "reward_earned": true,
            "reward_amount": 500,  // in cents
            "activated_at": 1234567890,
            "created_at": 1234567890
        },
        ...
    ]
}

Errors:
- 404: No referral code found for user
```

### Public Endpoints

#### Track Referral Click
```
POST /referrals/track
Content-Type: application/json

Request Body:
{
    "code": "ref-john-abc123",
    "email": "newuser@example.com",
    "utm_source": "email",
    "utm_campaign": "launch",
    "utm_medium": "newsletter"
}

Response:
{
    "success": true,
    "data": {
        "tracked": true,
        "referrer_name": "John Doe",
        "reward_description": "10% off",
        "message": "Click tracked. You'll receive 10% off when you sign up!"
    }
}

Errors:
- 400: Invalid referral code
- 400: Email is required
```

#### Get Leaderboard
```
GET /referrals/leaderboard?limit=10

Response:
{
    "success": true,
    "leaderboard": [
        {
            "rank": 1,
            "referrer_api_key": "nl_abc123",
            "display_name": "John Doe",
            "referral_code": "ref-john-abc123",
            "successful_referrals": 15,
            "total_rewards_earned": 7500,  // in cents
            "total_rewards_value": "$75.00"
        },
        {
            "rank": 2,
            "referrer_api_key": "nl_xyz789",
            "display_name": "Jane Smith",
            "referral_code": "ref-jane-xyz789",
            "successful_referrals": 12,
            "total_rewards_earned": 6000,
            "total_rewards_value": "$60.00"
        },
        ...
    ]
}

Query Parameters:
- limit: Number of results (default: 10, max: 100)
```

#### Get Referral Details
```
GET /referrals/{code}

Response:
{
    "success": true,
    "data": {
        "referrer_name": "John Doe",
        "reward_description": "Get 10% off your first month",
        "active": true
    }
}

Errors:
- 404: Referral code not found
```

## Usage Examples

### cURL Examples

#### Generate a Referral Code
```bash
curl -X POST https://api.nanolambda.com/referrals/generate \
  -H "x-api-key: nl_1234567890abcdef" \
  -H "Content-Type: application/json" \
  -d '{
    "display_name": "My Awesome Referral",
    "reward_type": "percentage",
    "reward_amount": 15,
    "max_referrals": 50
  }'
```

#### Get Your Referral Code
```bash
curl https://api.nanolambda.com/referrals/my-code \
  -H "x-api-key: nl_1234567890abcdef"
```

#### Track a Referral Click
```bash
curl -X POST https://api.nanolambda.com/referrals/track \
  -H "Content-Type: application/json" \
  -d '{
    "code": "ref-john-abc123",
    "email": "newuser@example.com",
    "utm_source": "twitter",
    "utm_campaign": "promo_q4"
  }'
```

#### Get the Leaderboard
```bash
curl "https://api.nanolambda.com/referrals/leaderboard?limit=20"
```

#### View a Referral's Details
```bash
curl https://api.nanolambda.com/referrals/ref-john-abc123
```

#### Get Your Referral Rewards
```bash
curl https://api.nanolambda.com/referrals/my-rewards \
  -H "x-api-key: nl_1234567890abcdef"
```

## Dashboard Integration

### Referral Widget

The dashboard includes a "Share & Earn" button that opens a referral management dialog with:

1. **Referral Code Display**: Copy your unique referral code
2. **Statistics Dashboard**:
   - Total referrals brought in
   - Active referrals (signed up customers)
   - Pending referrals (tracked clicks not yet converted)
   - Total earnings from rewards
3. **Social Sharing**:
   - Email sharing with pre-filled message
   - Twitter integration with shareable tweet
   - LinkedIn sharing
   - Direct link copying

### Referral Code Generation

If you don't have a referral code yet, the dialog provides a "Generate Code" button that:
- Creates a unique referral code formatted as `ref-{name}-{random}`
- Sets default reward to 10% off
- Activates immediately
- Returns you to the referral dialog to start sharing

## Referral Workflow

### Step 1: Create Referral Code
User generates a unique referral code with reward configuration:
```
User → Dashboard → "Share & Earn" → "Generate Code"
```

### Step 2: Share the Code
User shares the referral code or link via:
- Direct link: `https://nanolambda.com/referrals/{code}`
- Social media with pre-filled message
- Email invitation
- Direct copy-paste to message

### Step 3: Track Click
When someone clicks or uses the referral code:
```
POST /referrals/track with code, email, utm_source
→ Creates referral_reward record with status='pending'
→ Returns referrer name and reward info
```

### Step 4: Activate Referral
When referred person signs up:
```
POST /accounts/signup with referral_code
→ System validates code hasn't exceeded max_referrals
→ Creates account and links to referral_reward
→ Updates referral_reward status='activated'
→ Increments current_referrals on referral_code
```

### Step 5: Claim Reward
When referrer wants to use reward:
```
POST /referrals/claim with referral_code_id
→ Creates discount code automatically
→ Links discount_code_id to referral_reward
→ Updates referral_reward status='claimed'
→ Referrer can now use discount on next payment
```

## Reward Mechanics

### Percentage-Based Rewards
- Calculated as percentage off next month's subscription
- Example: 10% reward on $50/month = $5 discount
- Automatically applied as discount code

### Fixed-Amount Rewards
- Fixed dollar credit toward subscription
- Example: $10 fixed reward = $10 credit
- Automatically applied as discount code

### Reward Caps
- Optional maximum number of referrals per code
- Prevents unlimited reward liability
- Once max_referrals reached, code becomes inactive
- Referrer can generate new code if needed

## Leaderboard Mechanics

### Ranking Algorithm
1. Referrers ranked by number of **successful referrals** (activated status)
2. Secondary sort by total rewards earned (descending)
3. Includes referrer name and code for transparency
4. Updates in real-time as new referrals activate

### Privacy
- Shows API key (for identification only)
- Shows user's display name (set when generating code)
- Shows public referral code
- Does NOT show personal email or sensitive info

## Integration with Discount System

The Referral Program integrates with the Discount Code system:

1. **Automatic Discount Creation**: When referral activates, system creates discount code
2. **Tracking**: discount_code_id linked in referral_reward record
3. **Usage**: Referrer's discount code automatically applied to next payment
4. **Auditing**: Can trace discount back to referral origin

## Data Structure Details

### ReferralCode Struct
```rust
pub struct ReferralCode {
    pub id: i64,
    pub referrer_api_key: String,  // Who created the code
    pub code: String,               // Unique code like "ref-john-abc123"
    pub display_name: String,       // User's display name
    pub reward_type: String,        // "percentage" or "fixed"
    pub reward_amount: i64,         // Amount in cents or percent
    pub reward_description: String, // "10% off" or "$10 off"
    pub max_referrals: Option<i64>,// Cap on referrals (null = unlimited)
    pub current_referrals: i64,     // Count of successful referrals
    pub active: bool,               // Can be used
    pub created_at: i64,
    pub updated_at: i64,
}
```

### ReferralReward Struct
```rust
pub struct ReferralReward {
    pub id: i64,
    pub referral_code_id: i64,      // Which code this came from
    pub referrer_api_key: String,   // Who benefits from this
    pub referred_api_key: Option<String>, // New customer's API key
    pub referred_email: String,      // New customer's email
    pub status: String,              // pending | activated | claimed
    pub reward_earned: bool,         // True if customer signed up
    pub reward_amount: Option<i64>,  // Calculated when activated
    pub discount_code_id: Option<String>, // Link to discount code
    pub tracking_data: String,       // JSON with UTM params
    pub referred_at: i64,            // When click tracked
    pub activated_at: Option<i64>,   // When customer signed up
    pub claimed_at: Option<i64>,     // When reward claimed
    pub created_at: i64,
    pub updated_at: i64,
}
```

### ReferralStats Struct
```rust
pub struct ReferralStats {
    pub total_referrals: i64,        // All tracked referrals
    pub active_referrals: i64,       // Signed up customers
    pub pending_referrals: i64,      // Tracked but not signed up
    pub total_rewards_earned: i64,   // Total earnings in cents
    pub total_rewards_value: String, // Formatted "$X.XX"
}
```

### ReferralLeaderboard Struct
```rust
pub struct ReferralLeaderboard {
    pub rank: i64,                   // 1-indexed position
    pub referrer_api_key: String,
    pub display_name: String,
    pub referral_code: String,
    pub successful_referrals: i64,   // Count of activated
    pub total_rewards_earned: i64,   // Total in cents
    pub total_rewards_value: String, // Formatted
}
```

## Implementation Details

### Code Generation
- Format: `ref-{display_name}-{random_6_char_string}`
- Example: `ref-john-doe-abc123`, `ref-startup-xyz789`
- Unique constraint ensures no collisions
- Lowercase for consistency

### Tracking Flow
1. User visits referral link: `https://nanolambda.com/referrals/{code}`
2. Click handler calls `POST /referrals/track` with code and email
3. System validates code exists and hasn't exceeded max
4. Creates referral_reward with status='pending'
5. Returns referrer info and expected reward to display

### Activation Flow
1. Referred person completes signup
2. Signup includes optional referral_code parameter
3. System validates code and checks max_referrals
4. Updates referral_reward: status='activated', activated_at=now
5. Increments current_referrals on referral_code
6. Calculates reward_amount based on reward_type and referred person's tier

### Claiming Flow
1. Referrer wants to use reward
2. Calls `POST /referrals/claim` with referral_code_id
3. System creates automatic discount code
4. Links discount code to referral_reward
5. Updates referral_reward: status='claimed'
6. Discount applied to next payment

## Security Considerations

### Email Validation
- Referral codes can only be used once per email
- Multiple attempts from same email are tracked
- Fraud detection: unusual click patterns flagged

### Rate Limiting
- Public endpoints (track, leaderboard) rate-limited to prevent abuse
- Protected endpoints limited per API key
- Burst protection on dashboard requests

### Data Privacy
- Referred person's email shown only to referrer (authenticated)
- Public leaderboard doesn't expose personal emails
- Tracking data stored but not exposed in API responses
- GDPR compliance: emails can be requested for deletion

### Reward Constraints
- Max cap per referral code prevents unlimited liability
- Referrer can only access own reward data
- System validates reward amounts match configuration

## Testing

### Unit Tests
The referral.rs module includes comprehensive tests:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_generate_referral_code() {
        // Test code generation with validation
    }

    #[tokio::test]
    async fn test_track_referral_click() {
        // Test click tracking with UTM parameters
    }

    #[tokio::test]
    async fn test_activate_referral() {
        // Test referral activation on signup
    }
}
```

### Manual Testing

**Test Referral Code Generation:**
```bash
# Generate referral code
curl -X POST http://localhost:3000/referrals/generate \
  -H "x-api-key: test_key_123" \
  -H "Content-Type: application/json" \
  -d '{"display_name": "Test User", "reward_type": "percentage", "reward_amount": 15}'

# Verify code was created
curl http://localhost:3000/referrals/my-code \
  -H "x-api-key: test_key_123"
```

**Test Click Tracking:**
```bash
curl -X POST http://localhost:3000/referrals/track \
  -H "Content-Type: application/json" \
  -d '{"code": "ref-test-abc123", "email": "referred@example.com", "utm_source": "test"}'
```

**Test Leaderboard:**
```bash
curl "http://localhost:3000/referrals/leaderboard?limit=5"
```

## File Structure

```
crates/
├── storage/src/
│   ├── referral.rs          # ReferralManager and schemas
│   └── lib.rs               # Export referral module
├── api-server/src/
│   ├── referral_handlers.rs # HTTP endpoint handlers
│   ├── lib.rs               # Routes and integration
│   └── dashboard.html       # UI with Share & Earn button
└── Cargo.toml               # Added chrono dependency
```

## Performance Considerations

### Database Indexes
- `idx_referrer_api_key`: Fast lookup of user's codes and rewards
- `idx_code`: Fast lookup by referral code
- `idx_status`: Efficient filtering for pending/activated
- `idx_active`: Quick active code queries for validation

### Query Patterns
- Get user's code: Index on referrer_api_key
- Track click: Index on code, validate max_referrals
- Activation: Index on status for statistics
- Leaderboard: Efficient aggregate query with pagination

### Caching Opportunities
- Leaderboard: Cache top 100 for 5 minutes
- User stats: Cache for user's session
- Code validation: In-memory cache of active codes

## Future Enhancements

### Phase 2 Improvements
1. **Tiered Rewards**: Different rewards based on referred customer's tier
2. **Viral Bonuses**: Extra rewards when referral count hits milestones (10, 50, 100)
3. **Team Referrals**: Referral pool for teams/organizations
4. **Referral Tiers**: Different reward rates based on referrer's history
5. **Expiration**: Codes can expire after X days/months
6. **Analytics**: Conversion funnel, click-to-signup ratio, reward ROI
7. **Bulk Share**: Generate multiple codes for specific campaigns
8. **A/B Testing**: Test different reward amounts and messaging

### Phase 3 Improvements
1. **Referral Events**: Webhook notifications on activation/claim
2. **Custom Branding**: Referrer's name prominently on landing page
3. **Referral API Webhooks**: Listen to referral events
4. **Affiliate Portal**: Full dashboard for top referrers
5. **Commission Tracking**: Real-time commission calculations
6. **Payout Integration**: Direct payouts to referrers (not just credits)

## Troubleshooting

### Referral Code Not Working
**Issue**: User claims referral code is invalid
**Solution**: 
1. Check code exists: `GET /referrals/{code}`
2. Verify code is active: `active=true` in response
3. Check max_referrals not exceeded: `current_referrals < max_referrals`

### Reward Not Appearing
**Issue**: Referrer doesn't see reward after referred signup
**Solution**:
1. Verify referral_reward.status='activated'
2. Check referred person's account was actually created
3. Confirm reward_amount was calculated correctly

### Leaderboard Not Updating
**Issue**: Referrer in leaderboard but count doesn't match
**Solution**:
1. Query needs to count status='activated' only
2. Pending referrals don't count yet
3. Cache may need refresh (5 minute TTL)

## API Code Examples

### Rust/Tokio Implementation
```rust
use nanolambda_storage::referral::ReferralManager;

let referral_mgr = ReferralManager::new(pool).await?;

// Generate code
let code = referral_mgr.generate_referral_code(
    "nl_123",
    "John Doe",
    "percentage",
    15,
    "15% off",
    Some(100),
).await?;

// Track click
let reward = referral_mgr.track_referral_click(
    code.id,
    "newuser@example.com",
    r#"{"utm_source":"email"}"#,
).await?;

// Get stats
let stats = referral_mgr.get_referral_stats("nl_123").await?;

// Get leaderboard
let leaderboard = referral_mgr.get_leaderboard(10).await?;
```

### JavaScript/Dashboard Integration
```javascript
// Show referral dialog
async function showReferralDialog() {
    const apiKey = localStorage.getItem('apiKey');
    const response = await fetch('/referrals/my-code', {
        headers: { 'x-api-key': apiKey }
    });
    const data = await response.json();
    // Render dialog with stats and sharing buttons
}

// Share referral
function shareVia(platform, code) {
    const shareUrl = `https://nanolambda.com/referrals/${code}`;
    // Open share modal based on platform
}
```

## Summary

The Referral Program System provides a complete viral growth mechanism with:
- Unique referral codes per user
- Click tracking with UTM parameters
- Automatic reward calculation and application
- Public leaderboard for social proof
- Dashboard integration for easy sharing
- Integration with discount code system
- Comprehensive API for programmatic access

This implementation will drive sustainable customer acquisition through network effects and word-of-mouth marketing.
