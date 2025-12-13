// Churn Analysis and Prevention System
// Identifies at-risk customers and provides intervention recommendations

use anyhow::Result;
use serde::{Deserialize, Serialize};
use sqlx::SqlitePool;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Mutex;

/// Churn risk assessment for a customer
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChurnRiskProfile {
    pub api_key: String,
    pub risk_score: f64,                    // 0-100, higher = more risk
    pub risk_level: String,                 // low/medium/high/critical
    pub likelihood_to_churn: f64,           // 0-1 probability
    pub days_until_predicted_churn: Option<i64>,
    pub primary_risk_factors: Vec<RiskFactor>,
    pub intervention_recommendations: Vec<Intervention>,
    pub estimated_value_at_risk: i64,       // CLV in cents
    pub last_analyzed: String,
}

/// Individual risk factor contributing to churn
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RiskFactor {
    pub factor: String,
    pub severity: String,                   // low/medium/high/critical
    pub impact_score: f64,                  // 0-100
    pub description: String,
}

/// Recommended intervention to prevent churn
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Intervention {
    pub action: String,
    pub priority: String,                   // low/medium/high/urgent
    pub estimated_cost: i64,                // cents
    pub expected_success_rate: f64,         // 0-1
    pub timeline: String,                   // immediate/1-week/1-month
    pub owner: String,                      // customer-success/sales/product
}

/// Churn prediction for a time period
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChurnPrediction {
    pub period: String,                     // next-week/next-month/next-quarter
    pub predicted_churns: i64,
    pub predicted_value_loss: i64,          // cents
    pub at_risk_customers: Vec<String>,     // api_keys
    pub confidence: f64,                    // 0-1
}

/// Historical churn event
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChurnEvent {
    pub api_key: String,
    pub churned_date: String,
    pub account_age_days: i64,
    pub total_revenue_lost: i64,
    pub churn_reason: Option<String>,
    pub warning_signs: Vec<String>,
    pub could_have_been_prevented: bool,
}

/// Platform churn metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlatformChurnMetrics {
    pub total_customers: i64,
    pub at_risk_count: i64,
    pub churn_rate_30d: f64,                // percentage
    pub churn_rate_90d: f64,
    pub avg_risk_score: f64,
    pub total_value_at_risk: i64,
    pub prevented_churns_count: i64,
    pub top_risk_factors: Vec<RiskFactor>,
}

/// Churn analysis manager
pub struct ChurnAnalyzer {
    // In-memory storage for MVP
    risk_profiles: Arc<Mutex<HashMap<String, ChurnRiskProfile>>>,
    churn_events: Arc<Mutex<Vec<ChurnEvent>>>,
    predictions: Arc<Mutex<HashMap<String, ChurnPrediction>>>,
    interventions_taken: Arc<Mutex<HashMap<String, Vec<Intervention>>>>,
    
    pool: SqlitePool,
}

impl ChurnAnalyzer {
    /// Create new churn analyzer
    pub async fn new(pool: SqlitePool) -> Result<Self> {
        Ok(Self {
            risk_profiles: Arc::new(Mutex::new(HashMap::new())),
            churn_events: Arc::new(Mutex::new(Vec::new())),
            predictions: Arc::new(Mutex::new(HashMap::new())),
            interventions_taken: Arc::new(Mutex::new(HashMap::new())),
            pool,
        })
    }

    /// Analyze churn risk for a customer
    pub async fn analyze_churn_risk(
        &self,
        api_key: &str,
        usage_declining: bool,
        payment_issues: i64,
        support_tickets: i64,
        last_login_days_ago: i64,
        feature_adoption_score: f64,        // 0-100
        nps_score: Option<i64>,             // -100 to 100
        predicted_ltv: i64,
    ) -> Result<ChurnRiskProfile> {
        let mut risk_factors = Vec::new();
        let mut risk_score = 0.0;

        // Factor 1: Usage decline (30 points max)
        if usage_declining {
            let impact = 30.0;
            risk_score += impact;
            risk_factors.push(RiskFactor {
                factor: "usage_decline".to_string(),
                severity: "high".to_string(),
                impact_score: impact,
                description: "Usage has declined over the past 30 days".to_string(),
            });
        }

        // Factor 2: Payment issues (25 points max)
        if payment_issues > 0 {
            let impact = (payment_issues as f64 * 8.0).min(25.0);
            let severity = if payment_issues >= 3 { "critical" } 
                          else if payment_issues >= 2 { "high" }
                          else { "medium" };
            risk_score += impact;
            risk_factors.push(RiskFactor {
                factor: "payment_failures".to_string(),
                severity: severity.to_string(),
                impact_score: impact,
                description: format!("{} payment failure(s) in recent history", payment_issues),
            });
        }

        // Factor 3: Support tickets (15 points max)
        if support_tickets > 5 {
            let impact = ((support_tickets - 5) as f64 * 3.0).min(15.0);
            risk_score += impact;
            risk_factors.push(RiskFactor {
                factor: "high_support_volume".to_string(),
                severity: "medium".to_string(),
                impact_score: impact,
                description: format!("{} support tickets - possible product issues", support_tickets),
            });
        }

        // Factor 4: Low engagement (20 points max)
        if last_login_days_ago > 14 {
            let impact = ((last_login_days_ago - 14) as f64 * 2.0).min(20.0);
            let severity = if last_login_days_ago > 30 { "high" } else { "medium" };
            risk_score += impact;
            risk_factors.push(RiskFactor {
                factor: "low_engagement".to_string(),
                severity: severity.to_string(),
                impact_score: impact,
                description: format!("Last login {} days ago", last_login_days_ago),
            });
        }

        // Factor 5: Low feature adoption (10 points max)
        if feature_adoption_score < 40.0 {
            let impact = ((40.0 - feature_adoption_score) / 4.0).min(10.0);
            risk_score += impact;
            risk_factors.push(RiskFactor {
                factor: "low_feature_adoption".to_string(),
                severity: "low".to_string(),
                impact_score: impact,
                description: format!("Using only {:.0}% of available features", feature_adoption_score),
            });
        }

        // Factor 6: Poor NPS score (10 points max)
        if let Some(nps) = nps_score {
            if nps < 0 {
                let impact = (nps.abs() as f64 / 10.0).min(10.0);
                risk_score += impact;
                risk_factors.push(RiskFactor {
                    factor: "negative_nps".to_string(),
                    severity: if nps < -50 { "high".to_string() } else { "medium".to_string() },
                    impact_score: impact,
                    description: format!("NPS score of {} indicates dissatisfaction", nps),
                });
            }
        }

        // Determine risk level
        let (risk_level, likelihood_to_churn) = if risk_score >= 70.0 {
            ("critical".to_string(), 0.85)
        } else if risk_score >= 50.0 {
            ("high".to_string(), 0.65)
        } else if risk_score >= 30.0 {
            ("medium".to_string(), 0.40)
        } else {
            ("low".to_string(), 0.15)
        };

        // Predict days until churn based on risk
        let days_until_predicted_churn = if risk_score >= 70.0 {
            Some(14) // 2 weeks
        } else if risk_score >= 50.0 {
            Some(30) // 1 month
        } else if risk_score >= 30.0 {
            Some(90) // 3 months
        } else {
            None // Low risk, no immediate churn predicted
        };

        // Generate intervention recommendations
        let interventions = self.generate_interventions(
            &risk_factors,
            risk_score,
            predicted_ltv,
        );

        let profile = ChurnRiskProfile {
            api_key: api_key.to_string(),
            risk_score,
            risk_level,
            likelihood_to_churn,
            days_until_predicted_churn,
            primary_risk_factors: risk_factors,
            intervention_recommendations: interventions,
            estimated_value_at_risk: predicted_ltv,
            last_analyzed: chrono::Utc::now().format("%Y-%m-%d %H:%M:%S").to_string(),
        };

        // Store profile
        let mut profiles = self.risk_profiles.lock().await;
        profiles.insert(api_key.to_string(), profile.clone());

        Ok(profile)
    }

    /// Generate intervention recommendations
    fn generate_interventions(
        &self,
        risk_factors: &[RiskFactor],
        risk_score: f64,
        predicted_ltv: i64,
    ) -> Vec<Intervention> {
        let mut interventions = Vec::new();
        let max_investment = (predicted_ltv as f64 * 0.15) as i64; // 15% of LTV

        for factor in risk_factors {
            match factor.factor.as_str() {
                "usage_decline" => {
                    interventions.push(Intervention {
                        action: "Schedule product training session".to_string(),
                        priority: "high".to_string(),
                        estimated_cost: 50000, // $500
                        expected_success_rate: 0.70,
                        timeline: "immediate".to_string(),
                        owner: "customer-success".to_string(),
                    });
                    interventions.push(Intervention {
                        action: "Offer usage consulting and best practices review".to_string(),
                        priority: "high".to_string(),
                        estimated_cost: 100000, // $1000
                        expected_success_rate: 0.65,
                        timeline: "1-week".to_string(),
                        owner: "customer-success".to_string(),
                    });
                }
                "payment_failures" => {
                    interventions.push(Intervention {
                        action: "Contact for payment method update".to_string(),
                        priority: "urgent".to_string(),
                        estimated_cost: 5000, // $50
                        expected_success_rate: 0.85,
                        timeline: "immediate".to_string(),
                        owner: "sales".to_string(),
                    });
                    if predicted_ltv > 500000 { // > $5,000
                        interventions.push(Intervention {
                            action: "Offer flexible payment terms or discount".to_string(),
                            priority: "urgent".to_string(),
                            estimated_cost: (predicted_ltv as f64 * 0.10) as i64,
                            expected_success_rate: 0.75,
                            timeline: "immediate".to_string(),
                            owner: "sales".to_string(),
                        });
                    }
                }
                "high_support_volume" => {
                    interventions.push(Intervention {
                        action: "Conduct product feedback session".to_string(),
                        priority: "medium".to_string(),
                        estimated_cost: 30000, // $300
                        expected_success_rate: 0.60,
                        timeline: "1-week".to_string(),
                        owner: "product".to_string(),
                    });
                    interventions.push(Intervention {
                        action: "Assign dedicated account manager".to_string(),
                        priority: if predicted_ltv > 1000000 { "high".to_string() } else { "medium".to_string() },
                        estimated_cost: 200000, // $2000/month
                        expected_success_rate: 0.80,
                        timeline: "immediate".to_string(),
                        owner: "customer-success".to_string(),
                    });
                }
                "low_engagement" => {
                    interventions.push(Intervention {
                        action: "Send re-engagement campaign with value reminders".to_string(),
                        priority: "medium".to_string(),
                        estimated_cost: 10000, // $100
                        expected_success_rate: 0.45,
                        timeline: "immediate".to_string(),
                        owner: "customer-success".to_string(),
                    });
                    interventions.push(Intervention {
                        action: "Offer new feature demo or use case consultation".to_string(),
                        priority: "medium".to_string(),
                        estimated_cost: 50000, // $500
                        expected_success_rate: 0.55,
                        timeline: "1-week".to_string(),
                        owner: "customer-success".to_string(),
                    });
                }
                "low_feature_adoption" => {
                    interventions.push(Intervention {
                        action: "Provide feature onboarding webinar".to_string(),
                        priority: "low".to_string(),
                        estimated_cost: 20000, // $200
                        expected_success_rate: 0.50,
                        timeline: "1-month".to_string(),
                        owner: "customer-success".to_string(),
                    });
                }
                "negative_nps" => {
                    interventions.push(Intervention {
                        action: "Executive escalation for feedback session".to_string(),
                        priority: if predicted_ltv > 1000000 { "urgent".to_string() } else { "high".to_string() },
                        estimated_cost: 100000, // $1000
                        expected_success_rate: 0.70,
                        timeline: "immediate".to_string(),
                        owner: "customer-success".to_string(),
                    });
                }
                _ => {}
            }
        }

        // Filter interventions by cost-effectiveness
        interventions.retain(|i| i.estimated_cost <= max_investment);

        // Sort by priority and success rate
        interventions.sort_by(|a, b| {
            let priority_order = |p: &str| match p {
                "urgent" => 0,
                "high" => 1,
                "medium" => 2,
                "low" => 3,
                _ => 4,
            };
            priority_order(&a.priority).cmp(&priority_order(&b.priority))
                .then(b.expected_success_rate.partial_cmp(&a.expected_success_rate).unwrap())
        });

        interventions
    }

    /// Get churn risk profile for a customer
    pub async fn get_risk_profile(&self, api_key: &str) -> Result<Option<ChurnRiskProfile>> {
        let profiles = self.risk_profiles.lock().await;
        Ok(profiles.get(api_key).cloned())
    }

    /// Record a churn event
    pub async fn record_churn(
        &self,
        api_key: &str,
        account_age_days: i64,
        total_revenue_lost: i64,
        churn_reason: Option<String>,
    ) -> Result<()> {
        let event = ChurnEvent {
            api_key: api_key.to_string(),
            churned_date: chrono::Utc::now().format("%Y-%m-%d").to_string(),
            account_age_days,
            total_revenue_lost,
            churn_reason,
            warning_signs: Vec::new(),
            could_have_been_prevented: false,
        };

        let mut events = self.churn_events.lock().await;
        events.push(event);

        // Remove from risk profiles
        let mut profiles = self.risk_profiles.lock().await;
        profiles.remove(api_key);

        Ok(())
    }

    /// Generate churn prediction for a period
    pub async fn predict_churn(&self, period: &str) -> Result<ChurnPrediction> {
        let profiles = self.risk_profiles.lock().await;
        
        let threshold = match period {
            "next-week" => 70.0,
            "next-month" => 50.0,
            "next-quarter" => 30.0,
            _ => 50.0,
        };

        let at_risk: Vec<_> = profiles.values()
            .filter(|p| p.risk_score >= threshold)
            .collect();

        let predicted_churns = at_risk.len() as i64;
        let predicted_value_loss: i64 = at_risk.iter()
            .map(|p| p.estimated_value_at_risk)
            .sum();
        let at_risk_customers: Vec<String> = at_risk.iter()
            .map(|p| p.api_key.clone())
            .collect();

        // Confidence based on data quality and sample size
        let confidence = if predicted_churns > 10 {
            0.80
        } else if predicted_churns > 5 {
            0.65
        } else {
            0.50
        };

        let prediction = ChurnPrediction {
            period: period.to_string(),
            predicted_churns,
            predicted_value_loss,
            at_risk_customers,
            confidence,
        };

        // Store prediction
        let mut predictions = self.predictions.lock().await;
        predictions.insert(period.to_string(), prediction.clone());

        Ok(prediction)
    }

    /// Get platform churn metrics
    pub async fn get_platform_metrics(&self) -> Result<PlatformChurnMetrics> {
        let profiles = self.risk_profiles.lock().await;
        let events = self.churn_events.lock().await;

        let total_customers = profiles.len() as i64;
        let at_risk_count = profiles.values()
            .filter(|p| p.risk_score >= 50.0)
            .count() as i64;

        let avg_risk_score = if total_customers > 0 {
            profiles.values().map(|p| p.risk_score).sum::<f64>() / total_customers as f64
        } else {
            0.0
        };

        let total_value_at_risk: i64 = profiles.values()
            .filter(|p| p.risk_score >= 50.0)
            .map(|p| p.estimated_value_at_risk)
            .sum();

        // Calculate churn rates
        let now = chrono::Utc::now();
        let churns_30d = events.iter()
            .filter(|e| {
                if let Ok(date) = chrono::NaiveDate::parse_from_str(&e.churned_date, "%Y-%m-%d") {
                    (now.date_naive() - date).num_days() <= 30
                } else {
                    false
                }
            })
            .count() as i64;

        let churns_90d = events.iter()
            .filter(|e| {
                if let Ok(date) = chrono::NaiveDate::parse_from_str(&e.churned_date, "%Y-%m-%d") {
                    (now.date_naive() - date).num_days() <= 90
                } else {
                    false
                }
            })
            .count() as i64;

        let churn_rate_30d = if total_customers > 0 {
            (churns_30d as f64 / total_customers as f64) * 100.0
        } else {
            0.0
        };

        let churn_rate_90d = if total_customers > 0 {
            (churns_90d as f64 / total_customers as f64) * 100.0
        } else {
            0.0
        };

        // Aggregate top risk factors
        let mut factor_counts: HashMap<String, i64> = HashMap::new();
        for profile in profiles.values() {
            for factor in &profile.primary_risk_factors {
                *factor_counts.entry(factor.factor.clone()).or_insert(0) += 1;
            }
        }

        let mut top_risk_factors: Vec<RiskFactor> = factor_counts.iter()
            .map(|(factor, count)| RiskFactor {
                factor: factor.clone(),
                severity: "medium".to_string(),
                impact_score: *count as f64,
                description: format!("Affecting {} customers", count),
            })
            .collect();

        top_risk_factors.sort_by(|a, b| b.impact_score.partial_cmp(&a.impact_score).unwrap());
        top_risk_factors.truncate(5);

        Ok(PlatformChurnMetrics {
            total_customers,
            at_risk_count,
            churn_rate_30d,
            churn_rate_90d,
            avg_risk_score,
            total_value_at_risk,
            prevented_churns_count: 0, // TODO: Track prevention successes
            top_risk_factors,
        })
    }

    /// Record an intervention taken
    pub async fn record_intervention(
        &self,
        api_key: &str,
        intervention: Intervention,
    ) -> Result<()> {
        let mut interventions = self.interventions_taken.lock().await;
        interventions.entry(api_key.to_string())
            .or_insert_with(Vec::new)
            .push(intervention);
        Ok(())
    }

    /// Get interventions taken for a customer
    pub async fn get_interventions(&self, api_key: &str) -> Result<Vec<Intervention>> {
        let interventions = self.interventions_taken.lock().await;
        Ok(interventions.get(api_key).cloned().unwrap_or_default())
    }
}
