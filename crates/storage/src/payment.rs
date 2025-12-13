use anyhow::{anyhow, Result};
use serde::{Deserialize, Serialize};
use sqlx::{SqlitePool, Row};
use std::sync::Arc;
use reqwest::Client;

use crate::tier::TierLevel;

// NOTE: This is a simplified payment manager. For production use:
// 1. Add async-stripe = "0.35" (or latest stable) to Cargo.toml
// 2. Replace mock API calls with real Stripe SDK calls
// 3. Add proper error handling and retry logic
// 4. Implement webhook signature verification
// 5. Add support for payment intents, setup intents, etc.

/// Payment manager for handling Stripe integration (simplified mock)
pub struct PaymentManager {
    http_client: Client,
    stripe_api_key: String,
    pool: SqlitePool,
}

/// Stripe customer and subscription information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CustomerInfo {
    pub api_key: String,
    pub stripe_customer_id: String,
    pub stripe_subscription_id: Option<String>,
    pub payment_method_id: Option<String>,
    pub subscription_status: Option<String>,
    pub created_at: i64,
    pub updated_at: i64,
}

/// Mock subscription (replace with real Stripe Subscription type)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MockSubscription {
    pub id: String,
    pub status: String,
}

/// Stripe price IDs for each tier (configured in Stripe Dashboard)
#[derive(Debug, Clone)]
pub struct StripePriceIds {
    pub starter_monthly: String,
    pub pro_monthly: String,
    pub enterprise_monthly: String,
}

impl StripePriceIds {
    /// Create from environment variables
    pub fn from_env() -> Result<Self> {
        Ok(Self {
            starter_monthly: std::env::var("STRIPE_PRICE_STARTER")
                .unwrap_or_else(|_| "price_starter_test".to_string()),
            pro_monthly: std::env::var("STRIPE_PRICE_PRO")
                .unwrap_or_else(|_| "price_pro_test".to_string()),
            enterprise_monthly: std::env::var("STRIPE_PRICE_ENTERPRISE")
                .unwrap_or_else(|_| "price_enterprise_test".to_string()),
        })
    }

    /// Get price ID for a tier
    pub fn get_price_id(&self, tier: &TierLevel) -> &str {
        match tier {
            TierLevel::Starter => &self.starter_monthly,
            TierLevel::Pro => &self.pro_monthly,
            TierLevel::Enterprise => &self.enterprise_monthly,
        }
    }
}

impl PaymentManager {
    /// Create a new payment manager
    pub async fn new(pool: SqlitePool, stripe_secret_key: String) -> Result<Self> {
        let http_client = Client::new();

        // Create database table if it doesn't exist
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS stripe_customers (
                api_key TEXT PRIMARY KEY,
                stripe_customer_id TEXT NOT NULL UNIQUE,
                stripe_subscription_id TEXT,
                payment_method_id TEXT,
                subscription_status TEXT,
                created_at INTEGER NOT NULL,
                updated_at INTEGER NOT NULL
            )
            "#,
        )
        .execute(&pool)
        .await?;

        Ok(Self {
            http_client,
            stripe_api_key: stripe_secret_key,
            pool,
        })
    }

    /// Create or retrieve Stripe customer for an API key
    pub async fn get_or_create_customer(
        &self,
        api_key: &str,
        email: Option<String>,
        name: Option<String>,
    ) -> Result<CustomerInfo> {
        // Check if customer already exists
        if let Some(customer) = self.get_customer(api_key).await? {
            return Ok(customer);
        }

        // TODO: Replace with real Stripe API call
        // For now, generate a mock customer ID
        let customer_id = format!("cus_{}", hex::encode(&api_key.as_bytes()[0..16]));
        
        tracing::info!(
            "Creating mock Stripe customer {} for API key (email: {:?}, name: {:?})",
            customer_id, email, name
        );

        let now = chrono::Utc::now().timestamp();
        let customer_info = CustomerInfo {
            api_key: api_key.to_string(),
            stripe_customer_id: customer_id,
            stripe_subscription_id: None,
            payment_method_id: None,
            subscription_status: None,
            created_at: now,
            updated_at: now,
        };

        // Save to database
        sqlx::query(
            r#"
            INSERT INTO stripe_customers 
            (api_key, stripe_customer_id, created_at, updated_at)
            VALUES (?, ?, ?, ?)
            "#,
        )
        .bind(&customer_info.api_key)
        .bind(&customer_info.stripe_customer_id)
        .bind(customer_info.created_at)
        .bind(customer_info.updated_at)
        .execute(&self.pool)
        .await?;

        Ok(customer_info)
    }

    /// Get customer info from database
    pub async fn get_customer(&self, api_key: &str) -> Result<Option<CustomerInfo>> {
        let row = sqlx::query(
            r#"
            SELECT api_key, stripe_customer_id, stripe_subscription_id, 
                   payment_method_id, subscription_status, created_at, updated_at
            FROM stripe_customers
            WHERE api_key = ?
            "#,
        )
        .bind(api_key)
        .fetch_optional(&self.pool)
        .await?;

        Ok(row.map(|row| CustomerInfo {
            api_key: row.get(0),
            stripe_customer_id: row.get(1),
            stripe_subscription_id: row.get(2),
            payment_method_id: row.get(3),
            subscription_status: row.get(4),
            created_at: row.get(5),
            updated_at: row.get(6),
        }))
    }

    /// Attach payment method to customer
    pub async fn attach_payment_method(
        &self,
        api_key: &str,
        payment_method_id: &str,
    ) -> Result<()> {
        let _customer = self
            .get_customer(api_key)
            .await?
            .ok_or_else(|| anyhow!("Customer not found"))?;

        // TODO: Replace with real Stripe API call to attach payment method
        tracing::info!(
            "Mock: Attaching payment method {} to customer",
            payment_method_id
        );

        // Update database
        let now = chrono::Utc::now().timestamp();
        sqlx::query(
            r#"
            UPDATE stripe_customers 
            SET payment_method_id = ?, updated_at = ?
            WHERE api_key = ?
            "#,
        )
        .bind(payment_method_id)
        .bind(now)
        .bind(api_key)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    /// Create a subscription for a tier
    pub async fn create_subscription(
        &self,
        api_key: &str,
        tier: &TierLevel,
        price_ids: &StripePriceIds,
    ) -> Result<MockSubscription> {
        let customer = self
            .get_customer(api_key)
            .await?
            .ok_or_else(|| anyhow!("Customer not found. Please create customer first."))?;

        if customer.payment_method_id.is_none() {
            return Err(anyhow!(
                "No payment method attached. Please add a payment method first."
            ));
        }

        let price_id = price_ids.get_price_id(tier);
        
        // TODO: Replace with real Stripe API call
        let subscription_id = format!("sub_{}", hex::encode(&api_key.as_bytes()[0..16]));
        tracing::info!(
            "Mock: Creating subscription {} for tier {:?} with price {}",
            subscription_id, tier, price_id
        );

        let subscription = MockSubscription {
            id: subscription_id.clone(),
            status: "active".to_string(),
        };

        // Update database
        let now = chrono::Utc::now().timestamp();
        sqlx::query(
            r#"
            UPDATE stripe_customers 
            SET stripe_subscription_id = ?, subscription_status = ?, updated_at = ?
            WHERE api_key = ?
            "#,
        )
        .bind(&subscription.id)
        .bind(&subscription.status)
        .bind(now)
        .bind(api_key)
        .execute(&self.pool)
        .await?;

        Ok(subscription)
    }

    /// Update subscription to a new tier
    pub async fn update_subscription(
        &self,
        api_key: &str,
        new_tier: &TierLevel,
        price_ids: &StripePriceIds,
    ) -> Result<MockSubscription> {
        let customer = self
            .get_customer(api_key)
            .await?
            .ok_or_else(|| anyhow!("Customer not found"))?;

        let subscription_id = customer
            .stripe_subscription_id
            .ok_or_else(|| anyhow!("No active subscription found"))?;

        let new_price_id = price_ids.get_price_id(new_tier);

        // TODO: Replace with real Stripe API call
        tracing::info!(
            "Mock: Updating subscription {} to new tier {:?} with price {}",
            subscription_id, new_tier, new_price_id
        );

        let updated_sub = MockSubscription {
            id: subscription_id,
            status: "active".to_string(),
        };

        // Update database
        let now = chrono::Utc::now().timestamp();
        sqlx::query(
            r#"
            UPDATE stripe_customers 
            SET subscription_status = ?, updated_at = ?
            WHERE api_key = ?
            "#,
        )
        .bind(&updated_sub.status)
        .bind(now)
        .bind(api_key)
        .execute(&self.pool)
        .await?;

        Ok(updated_sub)
    }

    /// Cancel a subscription
    pub async fn cancel_subscription(&self, api_key: &str) -> Result<MockSubscription> {
        let customer = self
            .get_customer(api_key)
            .await?
            .ok_or_else(|| anyhow!("Customer not found"))?;

        let subscription_id = customer
            .stripe_subscription_id
            .ok_or_else(|| anyhow!("No active subscription found"))?;

        // TODO: Replace with real Stripe API call
        tracing::info!("Mock: Canceling subscription {}", subscription_id);

        let canceled_sub = MockSubscription {
            id: subscription_id,
            status: "canceled".to_string(),
        };

        // Update database
        let now = chrono::Utc::now().timestamp();
        sqlx::query(
            r#"
            UPDATE stripe_customers 
            SET subscription_status = ?, updated_at = ?
            WHERE api_key = ?
            "#,
        )
        .bind(&canceled_sub.status)
        .bind(now)
        .bind(api_key)
        .execute(&self.pool)
        .await?;

        Ok(canceled_sub)
    }

    /// Handle Stripe webhook events
    pub async fn handle_webhook(
        &self,
        payload: &str,
        signature: &str,
        webhook_secret: &str,
    ) -> Result<()> {
        // TODO: Replace with real Stripe webhook verification
        tracing::info!(
            "Mock: Handling webhook (payload_len: {}, signature: {}, secret_len: {})",
            payload.len(), signature, webhook_secret.len()
        );
        
        // Parse webhook payload as JSON
        let event: serde_json::Value = serde_json::from_str(payload)?;
        
        if let Some(event_type) = event.get("type").and_then(|t| t.as_str()) {
            match event_type {
                "customer.subscription.created" | "customer.subscription.updated" => {
                    tracing::info!("Subscription event: {}", event_type);
                    // TODO: Handle subscription updates
                }
                "customer.subscription.deleted" => {
                    tracing::info!("Subscription canceled event");
                    // TODO: Handle subscription cancellation
                }
                "payment_intent.succeeded" => {
                    tracing::info!("Payment succeeded event");
                }
                "payment_intent.payment_failed" => {
                    tracing::warn!("Payment failed event");
                }
                _ => {
                    tracing::debug!("Unhandled webhook event: {}", event_type);
                }
            }
        }

        Ok(())
    }

}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_price_ids_from_env() {
        std::env::set_var("STRIPE_PRICE_STARTER", "price_starter_real");
        std::env::set_var("STRIPE_PRICE_PRO", "price_pro_real");
        std::env::set_var("STRIPE_PRICE_ENTERPRISE", "price_enterprise_real");

        let price_ids = StripePriceIds::from_env().unwrap();
        assert_eq!(price_ids.starter_monthly, "price_starter_real");
        assert_eq!(price_ids.pro_monthly, "price_pro_real");
        assert_eq!(price_ids.enterprise_monthly, "price_enterprise_real");

        assert_eq!(
            price_ids.get_price_id(&TierLevel::Starter),
            "price_starter_real"
        );
        assert_eq!(price_ids.get_price_id(&TierLevel::Pro), "price_pro_real");
        assert_eq!(
            price_ids.get_price_id(&TierLevel::Enterprise),
            "price_enterprise_real"
        );
    }
}
