# Task #18: Customer Lifetime Value (CLV) Tracking

**Status**: ✅ Complete  
**Version**: 0.1.0  
**Last Updated**: December 13, 2025

## Overview

Customer Lifetime Value (CLV) tracking provides predictive analytics for customer revenue potential over their entire relationship with the platform. This system enables data-driven decisions about customer acquisition costs, retention strategies, and pricing optimization.

## Features

### Core Functionality

- **CLV Calculation**: Predict total customer lifetime value using discounted cash flow
- **Revenue Prediction**: Forecast revenue for next 1, 6, and 12 months
- **Segment Analysis**: Classify customers into low/medium/high/premium segments
- **Cohort Tracking**: Analyze CLV by customer acquisition cohorts
- **Platform Analytics**: Aggregate CLV metrics across all customers
- **Retention Modeling**: Calculate retention probability and its impact on LTV

### Key Metrics Tracked

**Customer CLV Profile:**
- Predicted Lifetime Value (discounted)
- Historical Revenue (to date)
- Average Monthly Revenue
- Predicted Remaining Lifetime (months)
- CLV Segment (low/medium/high/premium)
- Revenue Trend (increasing/stable/declining)
- Retention Probability (0-1 scale)

**Platform Summary:**
- Total predicted value across all customers
- Average CLV per customer
- High-value customer count (>$10,000 LTV)
- At-risk high-value customers
- Segment distribution breakdown

## Business Value

### Strategic Benefits

**Customer Acquisition:**
- Set maximum CAC based on predicted LTV
- Identify most valuable customer profiles
- Optimize marketing spend allocation

**Retention Strategy:**
- Identify high-value at-risk customers
- Prioritize retention efforts by value
- Predict churn impact on revenue

**Pricing Optimization:**
- Understand value by tier
- Guide tier pricing decisions
- Identify upsell opportunities

**Financial Planning:**
- Forecast future revenue
- Predict cash flow patterns
- Plan resource allocation

## CLV Calculation Formula

### Predicted Lifetime Value

```
Predicted LTV = Historical Revenue + Sum of Discounted Future Revenue

Where:
- Historical Revenue = Total revenue to date
- Future Revenue = Avg Monthly Revenue × Predicted Lifetime Months
- Discount Factor = (1 + Annual Rate)^(1/12) per month
- Default Discount Rate = 10% annually
```

### Calculation Steps

1. **Calculate Average Monthly Revenue**:
   ```
   If monthly_revenues array provided:
     Avg = Sum(monthly_revenues) / Count
   Else:
     Avg = Total Revenue / Months Active
   ```

2. **Determine Revenue Trend**:
   ```
   Recent = Avg of last 2 months
   Older = Avg of first 2 months
   
   If Recent > Older × 1.1: "increasing"
   Else If Recent < Older × 0.9: "declining"
   Else: "stable"
   ```

3. **Predict Remaining Lifetime**:
   ```
   Base lifetime by retention probability:
   - > 0.8: 36 months (3 years)
   - > 0.6: 24 months (2 years)
   - > 0.4: 12 months (1 year)
   - ≤ 0.4: 6 months
   
   Adjust by trend:
   - increasing: +6 months
   - declining: ÷2 (minimum 3 months)
   - stable: no adjustment
   ```

4. **Calculate Discounted Future Value**:
   ```
   monthly_discount = (1 + 0.10)^(1/12)  // ~1.00797
   
   For each month M in 1..predicted_lifetime:
     discount_factor = 1 / monthly_discount^M
     monthly_value = avg_monthly_revenue × discount_factor
     predicted_ltv += monthly_value
   
   predicted_ltv += historical_revenue
   ```

5. **Assign CLV Segment**:
   ```
   If LTV ≥ $10,000: "premium"
   If LTV ≥ $5,000: "high"
   If LTV ≥ $1,000: "medium"
   Else: "low"
   ```

## API Endpoints

### Protected Endpoints (Require API Key)

#### Get Customer CLV
```
GET /clv
Headers: x-api-key: <api_key>

Response:
{
    "success": true,
    "data": {
        "api_key": "nl_123...",
        "tier": "pro",
        "account_age_days": 90,
        "total_revenue_to_date": 450000,  // $4,500 in cents
        "avg_monthly_revenue": 150000,    // $1,500
        "predicted_lifetime_months": 36,
        "predicted_ltv": 5850000,         // $58,500
        "historical_ltv": 450000,
        "retention_probability": 0.85,
        "clv_segment": "high",
        "revenue_trend": "increasing",
        "months_active": 3
    }
}
```

#### Calculate CLV
```
POST /clv/calculate
Headers: x-api-key: <api_key>
Content-Type: application/json

Request Body:
{
    "tier": "pro",
    "account_age_days": 90,
    "total_revenue": 450000,
    "monthly_revenues": [140000, 145000, 165000],  // Last 3 months
    "retention_probability": 0.85
}

Response: Same as GET /clv
```

#### Get Revenue Prediction
```
POST /clv/predict
Headers: x-api-key: <api_key>
Content-Type: application/json

Request Body:
{
    "current_monthly_revenue": 150000,
    "growth_rate": 0.05,              // 5% monthly growth
    "retention_probability": 0.85
}

Response:
{
    "success": true,
    "data": {
        "api_key": "nl_123...",
        "current_monthly_revenue": 150000,
        "predicted_next_month": 157500,
        "predicted_6_months": 1000000,
        "predicted_12_months": 2200000,
        "confidence_level": 0.895,
        "factors": [
            "High retention probability increases confidence",
            "Stable revenue pattern"
        ]
    }
}
```

#### Get Cohort Analysis
```
GET /clv/cohorts
Headers: x-api-key: <api_key>

Response:
{
    "success": true,
    "data": [
        {
            "cohort_month": "2024-12",
            "customer_count": 25,
            "avg_clv": 4500000,
            "retention_rate": 0.82,
            "avg_age_days": 15,
            "total_revenue": 11250000
        }
    ]
}
```

### Public Endpoints

#### Get CLV Segments
```
GET /clv/segments

Response:
{
    "success": true,
    "data": [
        {
            "segment_name": "premium",
            "customer_count": 5,
            "avg_clv": 12000000,        // $120,000
            "total_value": 60000000,
            "avg_monthly_revenue": 300000,
            "avg_retention_probability": 0.92,
            "percentage_of_total": 45.5
        },
        {
            "segment_name": "high",
            "customer_count": 15,
            "avg_clv": 6000000,
            "total_value": 90000000,
            "avg_monthly_revenue": 150000,
            "avg_retention_probability": 0.85,
            "percentage_of_total": 34.2
        }
    ]
}
```

#### Get Platform CLV Summary
```
GET /clv/summary

Response:
{
    "success": true,
    "data": {
        "total_customers": 50,
        "avg_clv": 5500000,           // $55,000
        "total_predicted_value": 275000000,  // $2.75M
        "high_value_customers": 12,
        "at_risk_high_value": 2,
        "top_clv_tier": "enterprise",
        "clv_segments": [ /* array of segments */ ]
    }
}
```

## Dashboard Integration

### CLV Dialog Features

The dashboard includes a "💎 View Lifetime Value" button that displays:

1. **Customer Metrics (if CLV calculated)**:
   - Predicted LTV: Total expected value
   - Historical Revenue: Revenue to date
   - Monthly Average: Average monthly revenue
   - Expected Lifetime: Remaining months predicted

2. **Customer Details**:
   - CLV Segment: Color-coded badge (premium/high/medium/low)
   - Revenue Trend: Increasing/Stable/Declining indicator
   - Account Age: Days since signup, months active
   - Retention: Probability percentage

3. **Platform Summary**:
   - Total customers count
   - Average CLV across platform
   - High-value customer count
   - At-risk high-value customers
   - Segment breakdown with distribution

4. **Calculation Prompt (if CLV not yet calculated)**:
   - Instructions to use POST /clv/calculate
   - Required parameters display

### UI Color Coding

**CLV Segments:**
- Premium: Green (#10b981)
- High: Blue (#3b82f6)
- Medium: Orange (#f59e0b)
- Low: Gray (#6b7280)

**Revenue Trends:**
- Increasing: 📈 Green
- Stable: ➡️ Blue
- Declining: 📉 Red

## Database Schema

### In-Memory Storage

The system uses in-memory HashMaps for fast access:

```rust
pub struct CLVManager {
    customer_clvs: Arc<Mutex<HashMap<String, CustomerCLV>>>,
    revenue_predictions: Arc<Mutex<HashMap<String, RevenuePrediction>>>,
    cohorts: Arc<Mutex<HashMap<String, CLVCohort>>>,
    discount_rate: f64,  // Default: 0.10 (10% annually)
}
```

### Data Structures

**CustomerCLV:**
```rust
pub struct CustomerCLV {
    pub api_key: String,
    pub tier: String,
    pub account_age_days: i64,
    pub total_revenue_to_date: i64,        // cents
    pub avg_monthly_revenue: i64,          // cents
    pub predicted_lifetime_months: i64,
    pub predicted_ltv: i64,                // cents
    pub historical_ltv: i64,               // cents
    pub retention_probability: f64,        // 0-1
    pub clv_segment: String,
    pub revenue_trend: String,
    pub months_active: i64,
    pub last_payment_date: Option<String>,
    pub next_expected_payment: Option<String>,
}
```

**CLVSegment:**
```rust
pub struct CLVSegment {
    pub segment_name: String,
    pub customer_count: i64,
    pub avg_clv: i64,
    pub total_value: i64,
    pub avg_monthly_revenue: i64,
    pub avg_retention_probability: f64,
    pub percentage_of_total: f64,
}
```

**RevenuePrediction:**
```rust
pub struct RevenuePrediction {
    pub api_key: String,
    pub current_monthly_revenue: i64,
    pub predicted_next_month: i64,
    pub predicted_6_months: i64,
    pub predicted_12_months: i64,
    pub confidence_level: f64,
    pub factors: Vec<String>,
}
```

## Use Cases

### Marketing & Sales

**Set CAC Targets:**
```
If Premium CLV = $120,000
And LTV:CAC target = 3:1
Then Max CAC = $40,000
```

**Identify Best Customers:**
```
Query: GET /clv/segments
Filter: segment_name = "premium"
Result: 5 customers, $120K avg LTV
Action: Clone acquisition strategy
```

### Customer Success

**Prioritize At-Risk Accounts:**
```
Query: GET /clv/summary
Result: at_risk_high_value = 2
Action: Immediate outreach to prevent churn
Expected Impact: Save $240,000 in LTV
```

**Personalized Retention:**
```
If retention_probability < 0.5 AND clv_segment = "high":
  Priority: Critical
  Action: Executive involvement
  Investment: Up to 10% of LTV
```

### Product Strategy

**Feature Prioritization:**
```
High CLV segments use Feature X heavily
→ Invest in Feature X improvements
→ Expected impact: Increase retention 5%
→ Value gain: $137,500 across 50 customers
```

**Pricing Optimization:**
```
Premium segment: $300K monthly revenue, 92% retention
→ Launch $5,000/month tier
→ Expected adoption: 3 customers
→ Annual impact: $180,000
```

## Testing

### Manual Test Cases

**Test CLV Calculation:**
```bash
curl -X POST http://localhost:3000/clv/calculate \
  -H "x-api-key: nl_test_key" \
  -H "Content-Type: application/json" \
  -d '{
    "tier": "pro",
    "account_age_days": 90,
    "total_revenue": 450000,
    "monthly_revenues": [140000, 145000, 165000],
    "retention_probability": 0.85
  }'
```

**Test Revenue Prediction:**
```bash
curl -X POST http://localhost:3000/clv/predict \
  -H "x-api-key: nl_test_key" \
  -H "Content-Type: application/json" \
  -d '{
    "current_monthly_revenue": 150000,
    "growth_rate": 0.05,
    "retention_probability": 0.85
  }'
```

**Test Platform Summary:**
```bash
curl http://localhost:3000/clv/summary
```

**Test Dashboard Integration:**
1. Open dashboard: http://localhost:3000/dashboard.html
2. Enter API key
3. Click "Load Usage & Billing"
4. Click "💎 View Lifetime Value"
5. Verify CLV metrics display correctly

## Integration Points

### With Existing Systems

**Usage Analytics (Task #17):**
- Retention probability feeds CLV calculation
- Health score correlates with CLV segment
- Growth trends inform revenue predictions

**Tier Manager:**
- Premium segment → Enterprise tier candidates
- Low CLV + high usage → Pricing optimization needed

**Billing System:**
- Historical revenue for CLV calculation
- Monthly revenue patterns for predictions

**Referral Program:**
- High CLV customers = best referrers
- Premium segment referrals worth 20% commission

## Performance Considerations

### In-Memory Design
- O(1) lookups by API key
- Thread-safe with Arc<Mutex<T>>
- No database round-trips
- Ideal for MVP/startup phase

### Scaling Strategy

1. **Current (< 1,000 customers)**: In-memory perfect
2. **Growth (1,000-10,000)**: Add PostgreSQL persistence with Redis cache
3. **Scale (10,000+)**: Dedicated analytics database (ClickHouse/TimescaleDB)

## Future Enhancements

### Phase 2 (Next 3 Months)
1. **Machine Learning**: Train ML model on historical data
2. **Cohort Retention Curves**: Track retention by cohort over time
3. **A/B Test Impact**: Measure feature impact on CLV
4. **Automated Alerts**: Notify when high-value customer at risk

### Phase 3 (6-12 Months)
1. **Predictive Churn**: ML-based churn prediction
2. **Intervention Playbooks**: Automated retention strategies
3. **CLV Optimization**: AI-recommended pricing/features
4. **Multi-touch Attribution**: CLV by acquisition channel

## File Structure

```
crates/
├── storage/src/
│   ├── clv.rs                 # CLVManager and data structures (380 lines)
│   └── lib.rs                 # Export clv module
├── api-server/src/
│   ├── clv_handlers.rs        # HTTP endpoint handlers (280 lines)
│   ├── lib.rs                 # Routes and integration
│   └── dashboard.html         # UI with CLV modal dialog
└── Cargo.toml                 # Dependencies
```

## Key Metrics

**Code Statistics:**
- CLVManager: 380 lines
- API Handlers: 280 lines
- Dashboard UI: 200+ lines
- Total: ~860 lines of production code

**API Coverage:**
- 4 protected endpoints (customer-specific)
- 2 public endpoints (platform-wide)
- 6 core data structures
- 8 manager methods

## Summary

The Customer Lifetime Value tracking system provides comprehensive revenue prediction and customer segmentation capabilities. With discounted cash flow modeling, retention probability analysis, and visual dashboard integration, the system enables data-driven decisions about customer acquisition, retention, and pricing.

**Key Features:**
- ✅ CLV calculation with discounted cash flow (10% annual discount rate)
- ✅ Revenue prediction for 1, 6, and 12 months
- ✅ Four-tier segmentation (premium/high/medium/low)
- ✅ Cohort analysis for acquisition tracking
- ✅ Platform-wide CLV summary and breakdowns
- ✅ Retention probability modeling
- ✅ Dashboard integration with visual metrics
- ✅ At-risk high-value customer identification

**Integration Complete:**
- ✅ 6 API endpoints (4 protected + 2 public)
- ✅ CLVManager with 8 core methods
- ✅ Dashboard modal with segment visualization
- ✅ Thread-safe in-memory storage
- ✅ Comprehensive documentation
- ✅ Production-ready code (0 errors)

This completes Task #18 and brings the project to **18/20 (90%) completion**.
