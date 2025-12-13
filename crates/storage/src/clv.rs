// Customer Lifetime Value (CLV) Manager
// Tracks and predicts customer lifetime value for business intelligence

use anyhow::Result;
use serde::{Deserialize, Serialize};
use sqlx::SqlitePool;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Mutex;

/// Customer Lifetime Value profile with revenue predictions
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CustomerCLV {
    pub api_key: String,
    pub tier: String,
    pub account_age_days: i64,
    pub total_revenue_to_date: i64,        // Total revenue in cents
    pub avg_monthly_revenue: i64,          // Average monthly revenue
    pub predicted_lifetime_months: i64,    // Expected remaining lifetime
    pub predicted_ltv: i64,                // Predicted lifetime value
    pub historical_ltv: i64,               // Actual revenue to date
    pub retention_probability: f64,        // 0-1, likelihood of retention
    pub clv_segment: String,               // low/medium/high/premium
    pub revenue_trend: String,             // increasing/stable/declining
    pub months_active: i64,
    pub last_payment_date: Option<String>,
    pub next_expected_payment: Option<String>,
}

/// CLV segment breakdown for cohort analysis
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CLVSegment {
    pub segment_name: String,              // low/medium/high/premium
    pub customer_count: i64,
    pub avg_clv: i64,
    pub total_value: i64,
    pub avg_monthly_revenue: i64,
    pub avg_retention_probability: f64,
    pub percentage_of_total: f64,
}

/// Revenue prediction model for a customer
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RevenuePrediction {
    pub api_key: String,
    pub current_monthly_revenue: i64,
    pub predicted_next_month: i64,
    pub predicted_6_months: i64,
    pub predicted_12_months: i64,
    pub confidence_level: f64,             // 0-1, prediction confidence
    pub factors: Vec<String>,              // Factors affecting prediction
}

/// Cohort analysis for CLV tracking
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CLVCohort {
    pub cohort_month: String,              // YYYY-MM format
    pub customer_count: i64,
    pub avg_clv: i64,
    pub retention_rate: f64,
    pub avg_age_days: i64,
    pub total_revenue: i64,
}

/// Platform CLV summary for executive dashboard
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlatformCLVSummary {
    pub total_customers: i64,
    pub avg_clv: i64,
    pub total_predicted_value: i64,
    pub high_value_customers: i64,         // CLV > $10,000
    pub at_risk_high_value: i64,           // High CLV but low retention
    pub top_clv_tier: String,
    pub clv_segments: Vec<CLVSegment>,
}

/// Manager for Customer Lifetime Value calculations
pub struct CLVManager {
    // In-memory storage for MVP (migrate to database later)
    customer_clvs: Arc<Mutex<HashMap<String, CustomerCLV>>>,
    revenue_predictions: Arc<Mutex<HashMap<String, RevenuePrediction>>>,
    cohorts: Arc<Mutex<HashMap<String, CLVCohort>>>,
    
    // Configuration
    discount_rate: f64,                     // Time value of money (default: 10%)
    pool: SqlitePool,
}

impl CLVManager {
    /// Create a new CLV manager
    pub async fn new(pool: SqlitePool) -> Result<Self> {
        Ok(Self {
            customer_clvs: Arc::new(Mutex::new(HashMap::new())),
            revenue_predictions: Arc::new(Mutex::new(HashMap::new())),
            cohorts: Arc::new(Mutex::new(HashMap::new())),
            discount_rate: 0.10, // 10% annual discount rate
            pool,
        })
    }

    /// Calculate CLV for a customer
    pub async fn calculate_clv(
        &self,
        api_key: &str,
        tier: &str,
        account_age_days: i64,
        total_revenue: i64,
        monthly_revenues: Vec<i64>,  // Last 6 months
        retention_probability: f64,
    ) -> Result<CustomerCLV> {
        let months_active = (account_age_days as f64 / 30.0).ceil() as i64;
        
        // Calculate average monthly revenue
        let avg_monthly_revenue = if !monthly_revenues.is_empty() {
            monthly_revenues.iter().sum::<i64>() / monthly_revenues.len() as i64
        } else {
            total_revenue / months_active.max(1)
        };

        // Calculate revenue trend
        let revenue_trend = if monthly_revenues.len() >= 3 {
            let recent = monthly_revenues[monthly_revenues.len() - 2..].iter().sum::<i64>() / 2;
            let older = monthly_revenues[0..2].iter().sum::<i64>() / 2;
            
            if recent > older * 110 / 100 {
                "increasing".to_string()
            } else if recent < older * 90 / 100 {
                "declining".to_string()
            } else {
                "stable".to_string()
            }
        } else {
            "insufficient_data".to_string()
        };

        // Predict remaining lifetime in months
        // Higher retention = longer expected lifetime
        let base_lifetime = if retention_probability > 0.8 {
            36 // 3 years for high retention
        } else if retention_probability > 0.6 {
            24 // 2 years for medium retention
        } else if retention_probability > 0.4 {
            12 // 1 year for fair retention
        } else {
            6  // 6 months for low retention
        };

        // Adjust based on trend
        let predicted_lifetime_months = match revenue_trend.as_str() {
            "increasing" => base_lifetime + 6,
            "declining" => (base_lifetime / 2).max(3),
            _ => base_lifetime,
        };

        // Calculate predicted LTV using discounted cash flow
        let mut predicted_ltv = total_revenue; // Start with historical
        let monthly_discount = (1.0 + self.discount_rate).powf(1.0 / 12.0);
        
        for month in 1..=predicted_lifetime_months {
            let discount_factor = 1.0 / monthly_discount.powi(month as i32);
            let monthly_value = (avg_monthly_revenue as f64 * discount_factor) as i64;
            predicted_ltv += monthly_value;
        }

        // Determine CLV segment
        let clv_segment = if predicted_ltv >= 1_000_000 {
            "premium".to_string() // $10,000+
        } else if predicted_ltv >= 500_000 {
            "high".to_string()    // $5,000+
        } else if predicted_ltv >= 100_000 {
            "medium".to_string()  // $1,000+
        } else {
            "low".to_string()     // < $1,000
        };

        let clv = CustomerCLV {
            api_key: api_key.to_string(),
            tier: tier.to_string(),
            account_age_days,
            total_revenue_to_date: total_revenue,
            avg_monthly_revenue,
            predicted_lifetime_months,
            predicted_ltv,
            historical_ltv: total_revenue,
            retention_probability,
            clv_segment,
            revenue_trend,
            months_active,
            last_payment_date: None,
            next_expected_payment: None,
        };

        // Store in memory
        let mut clvs = self.customer_clvs.lock().await;
        clvs.insert(api_key.to_string(), clv.clone());

        Ok(clv)
    }

    /// Get CLV for a customer
    pub async fn get_customer_clv(&self, api_key: &str) -> Result<Option<CustomerCLV>> {
        let clvs = self.customer_clvs.lock().await;
        Ok(clvs.get(api_key).cloned())
    }

    /// Generate revenue prediction for next 12 months
    pub async fn predict_revenue(
        &self,
        api_key: &str,
        current_monthly_revenue: i64,
        growth_rate: f64, // Monthly growth rate (-1.0 to 1.0)
        retention_probability: f64,
    ) -> Result<RevenuePrediction> {
        let mut factors = Vec::new();

        // Adjust predictions based on retention
        let retention_factor = if retention_probability < 0.5 {
            factors.push("Low retention probability reduces confidence".to_string());
            0.5
        } else if retention_probability > 0.8 {
            factors.push("High retention probability increases confidence".to_string());
            1.0
        } else {
            0.8
        };

        // Growth trend factor
        let growth_factor = if growth_rate > 0.1 {
            factors.push("Strong growth trend detected".to_string());
            1.0 + growth_rate
        } else if growth_rate < -0.1 {
            factors.push("Declining revenue trend".to_string());
            1.0 + growth_rate
        } else {
            factors.push("Stable revenue pattern".to_string());
            1.0
        };

        // Calculate predictions
        let base_revenue = (current_monthly_revenue as f64 * retention_factor) as i64;
        
        let predicted_next_month = (base_revenue as f64 * growth_factor) as i64;
        let predicted_6_months = (base_revenue as f64 * growth_factor.powi(6) * 6.0) as i64;
        let predicted_12_months = (base_revenue as f64 * growth_factor.powi(12) * 12.0) as i64;

        // Confidence level based on data quality
        let confidence_level = retention_probability * 0.7 + 0.3;

        let prediction = RevenuePrediction {
            api_key: api_key.to_string(),
            current_monthly_revenue,
            predicted_next_month,
            predicted_6_months,
            predicted_12_months,
            confidence_level,
            factors,
        };

        // Store prediction
        let mut predictions = self.revenue_predictions.lock().await;
        predictions.insert(api_key.to_string(), prediction.clone());

        Ok(prediction)
    }

    /// Get CLV segments breakdown
    pub async fn get_clv_segments(&self) -> Result<Vec<CLVSegment>> {
        let clvs = self.customer_clvs.lock().await;
        
        let mut segments: HashMap<String, Vec<&CustomerCLV>> = HashMap::new();
        for clv in clvs.values() {
            segments.entry(clv.clv_segment.clone())
                .or_insert_with(Vec::new)
                .push(clv);
        }

        let total_value: i64 = clvs.values().map(|c| c.predicted_ltv).sum();
        let total_customers = clvs.len() as i64;

        let mut result = Vec::new();
        for (segment_name, customers) in segments {
            let customer_count = customers.len() as i64;
            let segment_total_value: i64 = customers.iter().map(|c| c.predicted_ltv).sum();
            let avg_clv = if customer_count > 0 {
                segment_total_value / customer_count
            } else {
                0
            };
            let avg_monthly_revenue = if customer_count > 0 {
                customers.iter().map(|c| c.avg_monthly_revenue).sum::<i64>() / customer_count
            } else {
                0
            };
            let avg_retention = if customer_count > 0 {
                customers.iter().map(|c| c.retention_probability).sum::<f64>() / customer_count as f64
            } else {
                0.0
            };
            let percentage = if total_value > 0 {
                (segment_total_value as f64 / total_value as f64) * 100.0
            } else {
                0.0
            };

            result.push(CLVSegment {
                segment_name,
                customer_count,
                avg_clv,
                total_value: segment_total_value,
                avg_monthly_revenue,
                avg_retention_probability: avg_retention,
                percentage_of_total: percentage,
            });
        }

        // Sort by average CLV (descending)
        result.sort_by(|a, b| b.avg_clv.cmp(&a.avg_clv));

        Ok(result)
    }

    /// Get platform-wide CLV summary
    pub async fn get_platform_clv_summary(&self) -> Result<PlatformCLVSummary> {
        let clvs = self.customer_clvs.lock().await;
        
        let total_customers = clvs.len() as i64;
        let total_predicted_value: i64 = clvs.values().map(|c| c.predicted_ltv).sum();
        let avg_clv = if total_customers > 0 {
            total_predicted_value / total_customers
        } else {
            0
        };

        // High value customers (>$10,000 LTV = >1,000,000 cents)
        let high_value_customers = clvs.values()
            .filter(|c| c.predicted_ltv >= 1_000_000)
            .count() as i64;

        // At-risk high value (high CLV but low retention)
        let at_risk_high_value = clvs.values()
            .filter(|c| c.predicted_ltv >= 1_000_000 && c.retention_probability < 0.5)
            .count() as i64;

        // Find top tier by total CLV
        let mut tier_values: HashMap<String, i64> = HashMap::new();
        for clv in clvs.values() {
            *tier_values.entry(clv.tier.clone()).or_insert(0) += clv.predicted_ltv;
        }
        let top_clv_tier = tier_values.iter()
            .max_by_key(|(_, value)| *value)
            .map(|(tier, _)| tier.clone())
            .unwrap_or_else(|| "none".to_string());

        // Get segments
        drop(clvs); // Release lock before calling get_clv_segments
        let clv_segments = self.get_clv_segments().await?;

        Ok(PlatformCLVSummary {
            total_customers,
            avg_clv,
            total_predicted_value,
            high_value_customers,
            at_risk_high_value,
            top_clv_tier,
            clv_segments,
        })
    }

    /// Create or update a cohort
    pub async fn record_cohort(
        &self,
        cohort_month: &str,
        customer_count: i64,
        avg_clv: i64,
        retention_rate: f64,
        avg_age_days: i64,
        total_revenue: i64,
    ) -> Result<()> {
        let cohort = CLVCohort {
            cohort_month: cohort_month.to_string(),
            customer_count,
            avg_clv,
            retention_rate,
            avg_age_days,
            total_revenue,
        };

        let mut cohorts = self.cohorts.lock().await;
        cohorts.insert(cohort_month.to_string(), cohort);

        Ok(())
    }

    /// Get cohort analysis
    pub async fn get_cohorts(&self) -> Result<Vec<CLVCohort>> {
        let cohorts = self.cohorts.lock().await;
        let mut result: Vec<CLVCohort> = cohorts.values().cloned().collect();
        
        // Sort by cohort month (most recent first)
        result.sort_by(|a, b| b.cohort_month.cmp(&a.cohort_month));
        
        Ok(result)
    }
}
