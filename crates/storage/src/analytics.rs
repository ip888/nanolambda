// Usage Analytics - Comprehensive insights into customer usage patterns
use crate::Result;
use serde::{Deserialize, Serialize};
use std::sync::{Arc, Mutex};
use std::collections::HashMap;

/// Daily usage summary for an API key
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DailyUsageSummary {
    pub api_key: String,
    pub date: String,                    // YYYY-MM-DD format
    pub total_invocations: i64,
    pub successful_invocations: i64,
    pub failed_invocations: i64,
    pub total_execution_time_ms: i64,
    pub avg_execution_time_ms: f64,
    pub total_memory_mb: f64,
    pub cold_starts: i64,
    pub error_rate: f64,
    pub cold_start_rate: f64,
}

/// Monthly usage trends
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MonthlyUsageTrend {
    pub api_key: String,
    pub month: String,                   // YYYY-MM format
    pub total_invocations: i64,
    pub growth_vs_previous: f64,         // percentage
    pub peak_day_invocations: i64,
    pub avg_daily_invocations: f64,
    pub most_used_function: Option<String>,
    pub total_cost_estimate: i64,        // in cents
}

/// Cost breakdown by function
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FunctionCostBreakdown {
    pub api_key: String,
    pub function_name: String,
    pub invocation_count: i64,
    pub execution_time_ms: i64,
    pub memory_gb_seconds: f64,
    pub estimated_cost: i64,             // in cents
    pub percentage_of_total: f64,
}

/// Customer usage profile
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UsageProfile {
    pub api_key: String,
    pub tier: String,
    pub account_age_days: i64,
    pub total_invocations: i64,
    pub monthly_growth_rate: f64,        // percentage
    pub is_growing: bool,
    pub is_declining: bool,
    pub churn_risk: f64,                 // 0-1, higher = more risk
    pub health_score: f64,               // 0-100, higher = healthier
    pub recommendations: Vec<String>,
}

/// Usage analytics aggregation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UsageAnalyticsSnapshot {
    pub timestamp: i64,
    pub api_key: String,
    pub period_type: String,             // "daily", "weekly", "monthly"
    pub period_start: i64,
    pub period_end: i64,
    pub total_invocations: i64,
    pub average_daily_invocations: f64,
    pub peak_invocations_per_hour: i64,
    pub total_execution_hours: f64,
    pub total_cost: i64,                 // in cents
    pub estimated_next_month_cost: i64,  // in cents
    pub most_used_functions: Vec<(String, i64)>, // (function_name, count)
    pub error_count: i64,
    pub error_rate: f64,
}

/// Usage analytics manager
pub struct UsageAnalyticsManager {
    // Daily summaries indexed by api_key and date
    daily_summaries: Arc<Mutex<HashMap<String, Vec<DailyUsageSummary>>>>,
    
    // Function costs indexed by api_key
    function_costs: Arc<Mutex<HashMap<String, Vec<FunctionCostBreakdown>>>>,
    
    // Usage profiles for all active customers
    usage_profiles: Arc<Mutex<HashMap<String, UsageProfile>>>,
    
    // Monthly trends indexed by api_key
    monthly_trends: Arc<Mutex<HashMap<String, Vec<MonthlyUsageTrend>>>>,
    
    // Pricing constants (in cents per unit)
    cost_per_invocation: i64,             // 0.0000002 USD per invocation
    cost_per_gb_second: i64,              // 0.000016667 USD per GB-second
}

impl UsageAnalyticsManager {
    /// Create new analytics manager
    pub async fn new(_pool: sqlx::SqlitePool) -> Result<Self> {
        Ok(Self {
            daily_summaries: Arc::new(Mutex::new(HashMap::new())),
            function_costs: Arc::new(Mutex::new(HashMap::new())),
            usage_profiles: Arc::new(Mutex::new(HashMap::new())),
            monthly_trends: Arc::new(Mutex::new(HashMap::new())),
            cost_per_invocation: 1,        // 0.00002 cents per invocation
            cost_per_gb_second: 2,         // 0.0002 cents per GB-second
        })
    }

    /// Record daily usage summary
    pub async fn record_daily_usage(&self, summary: DailyUsageSummary) -> Result<()> {
        let mut summaries = self.daily_summaries.lock().unwrap();
        let key = summary.api_key.clone();
        
        summaries.entry(key).or_insert_with(Vec::new).push(summary);
        
        Ok(())
    }

    /// Get daily usage summaries for an API key
    pub async fn get_daily_summaries(
        &self,
        api_key: &str,
        days: i64,
    ) -> Result<Vec<DailyUsageSummary>> {
        let summaries = self.daily_summaries.lock().unwrap();
        
        Ok(summaries
            .get(api_key)
            .map(|v| v.iter().take(days as usize).cloned().collect())
            .unwrap_or_default())
    }

    /// Get monthly usage trend
    pub async fn get_monthly_trends(&self, api_key: &str) -> Result<Vec<MonthlyUsageTrend>> {
        let trends = self.monthly_trends.lock().unwrap();
        
        Ok(trends
            .get(api_key)
            .cloned()
            .unwrap_or_default())
    }

    /// Calculate usage profile for a customer
    pub async fn calculate_usage_profile(
        &self,
        api_key: &str,
        tier: &str,
        account_age_days: i64,
        current_monthly_invocations: i64,
        previous_monthly_invocations: i64,
    ) -> Result<UsageProfile> {
        let monthly_growth = if previous_monthly_invocations > 0 {
            ((current_monthly_invocations - previous_monthly_invocations) as f64
                / previous_monthly_invocations as f64)
                * 100.0
        } else {
            0.0
        };

        let is_growing = monthly_growth > 5.0;
        let is_declining = monthly_growth < -10.0;

        // Calculate churn risk (0-1)
        let churn_risk = if is_declining { 0.7 } else if is_growing { 0.1 } else { 0.3 };

        // Calculate health score (0-100)
        let health_score = 100.0
            - (churn_risk * 50.0)
            + if account_age_days > 90 { 10.0 } else { 0.0 }
            + if is_growing { 15.0 } else { 0.0 };

        let mut recommendations = Vec::new();

        if is_declining {
            recommendations.push("Usage is declining. Consider optimizing your functions or reaching out for support.".to_string());
        }

        if monthly_growth > 100.0 {
            recommendations.push("Rapid growth detected. Ensure your tier aligns with usage needs.".to_string());
        }

        if account_age_days < 30 {
            recommendations.push("New customer. Review our documentation to maximize value.".to_string());
        }

        if is_growing && current_monthly_invocations > 1_000_000 {
            recommendations.push("High volume approaching tier limits. Consider upgrading tier.".to_string());
        }

        let profile = UsageProfile {
            api_key: api_key.to_string(),
            tier: tier.to_string(),
            account_age_days,
            total_invocations: current_monthly_invocations,
            monthly_growth_rate: monthly_growth,
            is_growing,
            is_declining,
            churn_risk,
            health_score,
            recommendations,
        };

        Ok(profile)
    }

    /// Calculate function cost breakdown
    pub async fn calculate_function_costs(
        &self,
        api_key: &str,
        functions: Vec<(String, i64, i64, f64)>, // (name, invocations, execution_ms, memory_gb)
    ) -> Result<Vec<FunctionCostBreakdown>> {
        let mut total_cost = 0i64;
        let mut breakdowns = Vec::new();

        for (name, invocations, execution_ms, memory_gb) in functions.iter() {
            let invocation_cost = invocations * self.cost_per_invocation;
            let gb_seconds = ((execution_ms * (*memory_gb * 1000.0) as i64) / 1000) as i64;
            let memory_cost = gb_seconds * self.cost_per_gb_second;
            let cost = invocation_cost + memory_cost;
            
            total_cost += cost;
            breakdowns.push((name.clone(), cost));
        }

        let mut results = Vec::new();
        for (i, (name, invocations, execution_ms, memory_gb)) in functions.iter().enumerate() {
            let cost = breakdowns[i].1;
            let percentage = if total_cost > 0 {
                (cost as f64 / total_cost as f64) * 100.0
            } else {
                0.0
            };

            let gb_seconds = (*execution_ms as f64 * memory_gb) / 1000.0;

            results.push(FunctionCostBreakdown {
                api_key: api_key.to_string(),
                function_name: name.clone(),
                invocation_count: *invocations,
                execution_time_ms: *execution_ms,
                memory_gb_seconds: gb_seconds,
                estimated_cost: cost,
                percentage_of_total: percentage,
            });
        }

        Ok(results)
    }

    /// Get comprehensive usage analytics snapshot
    pub async fn get_usage_analytics_snapshot(
        &self,
        api_key: &str,
        period_type: &str,
        invocations: i64,
        execution_hours: f64,
        errors: i64,
        functions: Vec<(String, i64)>,
    ) -> Result<UsageAnalyticsSnapshot> {
        let now = chrono::Utc::now().timestamp();
        
        // Estimate peak invocations per hour
        let peak_invocations = if execution_hours > 0.0 {
            (invocations as f64 / execution_hours) as i64
        } else {
            0
        };

        // Calculate average daily invocations based on period
        let avg_daily = match period_type {
            "daily" => invocations as f64,
            "weekly" => invocations as f64 / 7.0,
            "monthly" => invocations as f64 / 30.0,
            _ => invocations as f64,
        };

        // Estimate cost
        let invocation_cost = invocations * self.cost_per_invocation;
        let memory_cost = (execution_hours * 3600.0 * 1.0) as i64 * self.cost_per_gb_second / 3600;
        let total_cost = invocation_cost + memory_cost;

        // Project next month
        let estimated_next_month = total_cost * 30; // Simple projection

        let error_rate = if invocations > 0 {
            (errors as f64 / invocations as f64) * 100.0
        } else {
            0.0
        };

        Ok(UsageAnalyticsSnapshot {
            timestamp: now,
            api_key: api_key.to_string(),
            period_type: period_type.to_string(),
            period_start: now - 86400,
            period_end: now,
            total_invocations: invocations,
            average_daily_invocations: avg_daily,
            peak_invocations_per_hour: peak_invocations,
            total_execution_hours: execution_hours,
            total_cost,
            estimated_next_month_cost: estimated_next_month,
            most_used_functions: functions,
            error_count: errors,
            error_rate,
        })
    }

    /// Get usage summary for all customers (admin view)
    pub async fn get_platform_analytics_summary(
        &self,
    ) -> Result<PlatformAnalyticsSummary> {
        let profiles = self.usage_profiles.lock().unwrap();
        
        let total_customers = profiles.len() as i64;
        let growing_customers = profiles.values().filter(|p| p.is_growing).count() as i64;
        let declining_customers = profiles.values().filter(|p| p.is_declining).count() as i64;
        let total_invocations: i64 = profiles.values().map(|p| p.total_invocations).sum();
        let avg_health: f64 = if !profiles.is_empty() {
            profiles.values().map(|p| p.health_score).sum::<f64>() / profiles.len() as f64
        } else {
            0.0
        };

        Ok(PlatformAnalyticsSummary {
            total_active_customers: total_customers,
            total_invocations_today: total_invocations,
            growing_customers,
            declining_customers,
            avg_health_score: avg_health,
            churn_risk_customers: profiles.values().filter(|p| p.churn_risk > 0.6).count() as i64,
        })
    }

    /// Update a usage profile
    pub async fn update_usage_profile(&self, profile: UsageProfile) -> Result<()> {
        let mut profiles = self.usage_profiles.lock().unwrap();
        profiles.insert(profile.api_key.clone(), profile);
        Ok(())
    }
}

/// Platform-wide analytics summary
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlatformAnalyticsSummary {
    pub total_active_customers: i64,
    pub total_invocations_today: i64,
    pub growing_customers: i64,
    pub declining_customers: i64,
    pub avg_health_score: f64,
    pub churn_risk_customers: i64,
}
