// Annual Billing Management - Using in-memory storage
use crate::Result;
use serde::{Deserialize, Serialize};
use std::sync::{Arc, Mutex};
use std::collections::HashMap;
use chrono::Utc;

/// Annual billing plan configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnnualPlan {
    pub id: i64,
    pub api_key: String,
    pub tier: String,
    pub annual_price: i64,      // in cents
    pub monthly_equivalent: i64, // in cents (annual / 12)
    pub discount_percentage: i64,
    pub billing_start_date: i64,
    pub billing_end_date: i64,
    pub auto_renew: bool,
    pub status: String,  // "active", "cancelled", "expired"
    pub created_at: i64,
    pub updated_at: i64,
}

/// Annual billing with cost breakdown
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnnualBillingBreakdown {
    pub tier: String,
    pub monthly_price: i64,
    pub annual_regular_price: i64,
    pub discount_percentage: i64,
    pub discount_amount: i64,
    pub annual_discounted_price: i64,
    pub monthly_effective_price: i64,
    pub savings_per_month: i64,
    pub total_annual_savings: i64,
}

/// Annual subscription usage
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnnualSubscriptionUsage {
    pub api_key: String,
    pub tier: String,
    pub billing_period: String,
    pub days_remaining: i64,
    pub percentage_used: f64,
    pub renewal_date: i64,
    pub next_charge_amount: i64,
    pub can_downgrade: bool,
    pub downgrade_effective_date: Option<i64>,
}

/// Annual billing statistics
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct AnnualBillingStats {
    pub total_annual_subscribers: i64,
    pub total_annual_revenue: i64,
    pub average_discount_taken: f64,
    pub retention_rate: f64,
    pub churn_rate: f64,
    pub mrr_equivalent: i64,
}

/// Manager for annual billing operations
pub struct AnnualBillingManager {
    // In-memory storage for simplicity
    plans: Arc<Mutex<HashMap<String, AnnualPlan>>>,
    history: Arc<Mutex<Vec<(String, String, i64)>>>,
    next_id: Arc<Mutex<i64>>,
}

impl AnnualBillingManager {
    /// Initialize annual billing manager
    pub async fn new(_pool: sqlx::SqlitePool) -> Result<Self> {
        Ok(Self {
            plans: Arc::new(Mutex::new(HashMap::new())),
            history: Arc::new(Mutex::new(Vec::new())),
            next_id: Arc::new(Mutex::new(1)),
        })
    }

    /// Get billing breakdown for a tier
    pub async fn get_billing_breakdown(&self, tier: &str) -> Result<AnnualBillingBreakdown> {
        // Default tier pricing
        let monthly_price = match tier {
            "trial" => 0,
            "pro" => 2900,  // $29/month
            "enterprise" => 9900,  // $99/month
            _ => return Err(crate::error::StorageError::InvalidConfig("Unknown tier".to_string()).into()),
        };

        let annual_regular_price = monthly_price * 12;
        let discount_percentage = match tier {
            "trial" => 0,
            "pro" => 20,
            "enterprise" => 25,
            _ => 0,
        };

        let discount_amount = (annual_regular_price * discount_percentage) / 100;
        let annual_discounted_price = annual_regular_price - discount_amount;
        let monthly_effective_price = if annual_discounted_price > 0 {
            annual_discounted_price / 12
        } else {
            0
        };
        let savings_per_month = (monthly_price * discount_percentage) / 100;
        let total_annual_savings = discount_amount;

        Ok(AnnualBillingBreakdown {
            tier: tier.to_string(),
            monthly_price,
            annual_regular_price,
            discount_percentage,
            discount_amount,
            annual_discounted_price,
            monthly_effective_price,
            savings_per_month,
            total_annual_savings,
        })
    }

    /// Switch to annual billing for a customer
    pub async fn upgrade_to_annual(&self, api_key: &str, tier: &str) -> Result<AnnualPlan> {
        let breakdown = self.get_billing_breakdown(tier).await?;
        
        let now = Utc::now().timestamp();
        let billing_start_date = now;
        let billing_end_date = now + (365 * 24 * 60 * 60);

        let id = {
            let mut next_id = self.next_id.lock().unwrap();
            let id = *next_id;
            *next_id += 1;
            id
        };

        let plan = AnnualPlan {
            id,
            api_key: api_key.to_string(),
            tier: tier.to_string(),
            annual_price: breakdown.annual_discounted_price,
            monthly_equivalent: breakdown.monthly_effective_price,
            discount_percentage: breakdown.discount_percentage,
            billing_start_date,
            billing_end_date,
            auto_renew: true,
            status: "active".to_string(),
            created_at: now,
            updated_at: now,
        };

        self.plans.lock().unwrap().insert(api_key.to_string(), plan.clone());
        
        self.history.lock().unwrap().push((
            api_key.to_string(),
            "upgrade_to_annual".to_string(),
            now,
        ));

        Ok(plan)
    }

    /// Get annual plan for a customer
    pub async fn get_annual_plan(&self, api_key: &str) -> Result<Option<AnnualPlan>> {
        Ok(self.plans.lock().unwrap().get(api_key).cloned())
    }

    /// Get subscription usage for annual plan
    pub async fn get_annual_subscription_usage(&self, api_key: &str) -> Result<Option<AnnualSubscriptionUsage>> {
        let plan = match self.get_annual_plan(api_key).await? {
            Some(p) => p,
            None => return Ok(None),
        };

        let now = Utc::now().timestamp();
        let days_remaining = (plan.billing_end_date - now) / (24 * 60 * 60);
        let total_days = (plan.billing_end_date - plan.billing_start_date) / (24 * 60 * 60);
        let days_used = total_days - days_remaining;
        let percentage_used = if total_days > 0 {
            (days_used as f64 / total_days as f64) * 100.0
        } else {
            0.0
        };

        let billing_period = format!(
            "{} to {}",
            plan.billing_start_date,
            plan.billing_end_date
        );

        Ok(Some(AnnualSubscriptionUsage {
            api_key: api_key.to_string(),
            tier: plan.tier,
            billing_period,
            days_remaining,
            percentage_used,
            renewal_date: plan.billing_end_date,
            next_charge_amount: plan.annual_price,
            can_downgrade: days_remaining > 60,
            downgrade_effective_date: None,
        }))
    }

    /// Switch from annual back to monthly
    pub async fn downgrade_from_annual(&self, api_key: &str, reason: &str) -> Result<bool> {
        let now = Utc::now().timestamp();
        
        let mut plans = self.plans.lock().unwrap();
        if let Some(plan) = plans.get_mut(api_key) {
            plan.status = "cancelled".to_string();
            plan.updated_at = now;
            
            self.history.lock().unwrap().push((
                api_key.to_string(),
                "downgrade_from_annual".to_string(),
                now,
            ));
            
            Ok(true)
        } else {
            Ok(false)
        }
    }

    /// Get all active annual plans
    pub async fn get_active_annual_plans(&self) -> Result<Vec<AnnualPlan>> {
        let now = Utc::now().timestamp();
        let plans = self.plans.lock().unwrap();
        
        Ok(plans.values()
            .filter(|p| p.status == "active" && p.billing_end_date > now)
            .cloned()
            .collect())
    }

    /// Get annual billing statistics
    pub async fn get_annual_billing_stats(&self) -> Result<AnnualBillingStats> {
        let plans = self.plans.lock().unwrap();
        let now = Utc::now().timestamp();

        let active_plans: Vec<_> = plans.values()
            .filter(|p| p.status == "active" && p.billing_end_date > now)
            .collect();

        let total_annual_subscribers = active_plans.len() as i64;
        let total_annual_revenue: i64 = active_plans.iter().map(|p| p.annual_price).sum();
        let mrr_equivalent = if total_annual_subscribers > 0 {
            total_annual_revenue / 12
        } else {
            0
        };

        let avg_discount = if !active_plans.is_empty() {
            active_plans.iter().map(|p| p.discount_percentage as f64).sum::<f64>()
                / active_plans.len() as f64
        } else {
            0.0
        };

        let thirty_days_ago = now - (30 * 24 * 60 * 60);
        let churned = plans.values()
            .filter(|p| p.status == "cancelled" && p.updated_at > thirty_days_ago)
            .count() as i64;

        let total_all = plans.len() as i64;
        let churn_rate = if total_all > 0 {
            (churned as f64 / total_all as f64) * 100.0
        } else {
            0.0
        };

        let retention_rate = 100.0 - churn_rate;

        Ok(AnnualBillingStats {
            total_annual_subscribers,
            total_annual_revenue,
            average_discount_taken: avg_discount,
            retention_rate,
            churn_rate,
            mrr_equivalent,
        })
    }

    /// Process expiring annual plans
    pub async fn process_expiring_plans(&self) -> Result<(i64, i64)> {
        let now = Utc::now().timestamp();
        let renewal_window = 7 * 24 * 60 * 60;
        
        let mut renewed_count = 0i64;
        let mut renewal_amount = 0i64;

        let mut plans = self.plans.lock().unwrap();
        
        for (_key, plan) in plans.iter_mut() {
            if plan.status == "active" 
                && plan.auto_renew 
                && plan.billing_end_date >= now 
                && plan.billing_end_date <= now + renewal_window {
                
                plan.billing_end_date = now + (365 * 24 * 60 * 60);
                plan.updated_at = now;
                renewed_count += 1;
                renewal_amount += plan.annual_price;
            }
        }

        Ok((renewed_count, renewal_amount))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_annual_billing_breakdown() {
        // Would test pricing calculation
    }

    #[tokio::test]
    async fn test_upgrade_to_annual() {
        // Would test annual plan creation
    }
}
