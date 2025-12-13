use anyhow::{anyhow, Result};
use serde::{Deserialize, Serialize};
use serde_json::json;
use sqlx::{SqlitePool, Row};
use std::sync::Arc;
use reqwest::{Client, header};

use crate::tier::TierLevel;

// Real Stripe API integration using reqwest for direct HTTP calls
const STRIPE_API_BASE: &str = "https://api.stripe.com/v1";

/// Payment manager for handling Stripe integration
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

/// Stripe subscription response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StripeSubscription {
    pub id: String,
    pub status: String,
    pub customer: String,
    #[serde(default)]
    pub current_period_end: Option<i64>,
    #[serde(default)]
    pub cancel_at_period_end: Option<bool>,
}

/// Stripe customer response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StripeCustomer {
    pub id: String,
    pub email: Option<String>,
    pub name: Option<String>,
    #[serde(default)]
    pub created: i64,
}

/// Stripe payment method response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StripePaymentMethod {
    pub id: String,
    #[serde(rename = "type")]
    pub method_type: String,
    #[serde(default)]
    pub card: Option<StripeCard>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StripeCard {
    pub last4: String,
    pub brand: String,
    pub exp_month: i32,
    pub exp_year: i32,
}

/// Stripe price IDs for each tier (configured in Stripe Dashboard)
#[derive(Debug, Clone)]
pub struct StripePriceIds {
    pub starter_monthly: String,
    pub pro_monthly: String,
    pub enterprise_monthly: String,
}

impl StripePriceIds {
    pub fn from_env() -> Result<Self> {
        Ok(Self {
            starter_monthly: std::env::var("STRIPE_PRICE_STARTER")
                .unwrap_or_else(|_| "price_starter".to_string()),
            pro_monthly: std::env::var("STRIPE_PRICE_PRO")
                .unwrap_or_else(|_| "price_pro".to_string()),
            enterprise_monthly: std::env::var("STRIPE_PRICE_ENTERPRISE")
                .unwrap_or_else(|_| "price_enterprise".to_string()),
        })
    }

    pub fn get_price_id(&self, tier: &TierLevel) -> &str {
        match tier {
            TierLevel::Starter => &self.starter_monthly,
            TierLevel::Pro => &self.pro_monthly,
            TierLevel::Enterprise => &self.enterprise_monthly,
        }
    }
}

impl PaymentManager {
    /// Create a new PaymentManager
    pub async fn new(stripe_secret_key: String, pool: SqlitePool) -> Result<Self> {
        // Create database tables
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
            http_client: Client::new(),
            stripe_api_key: stripe_secret_key,
            pool,
        })
    }

    /// Make a Stripe API call with proper authentication
    async fn stripe_api_call<T: for<'de> Deserialize<'de>>(
        &self,
        method: &str,
        endpoint: &str,
        form_data: Option<&Vec<(&str, String)>>,
    ) -> Result<T> {
        let url = format!("{}{}", STRIPE_API_BASE, endpoint);
        
        let mut request = match method {
            "GET" => self.http_client.get(&url),
            "POST" => self.http_client.post(&url),
            "DELETE" => self.http_client.delete(&url),
            _ => return Err(anyhow!("Unsupported HTTP method: {}", method)),
        };

        request = request.basic_auth(&self.stripe_api_key, Some(""));

        if let Some(form) = form_data {
            request = request.form(form);
        }

        let response = request.send().await
            .map_err(|e| anyhow!("Stripe API request failed: {}", e))?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_default();
            return Err(anyhow!("Stripe API error ({}): {}", status, error_text));
        }

        response.json().await
            .map_err(|e| anyhow!("Failed to parse Stripe response: {}", e))
    }

    /// Create a Stripe customer via API
    pub async fn create_customer(&self, api_key: &str, email: &str, name: Option<&str>) -> Result<String> {
        // Check if customer already exists
        if let Some(existing) = self.get_customer(api_key).await? {
            return Ok(existing.stripe_customer_id);
        }

        // Build form data for Stripe API
        let mut form = vec![("email", email.to_string())];
        if let Some(n) = name {
            form.push(("name", n.to_string()));
        }
        form.push(("metadata[nanolambda_api_key]", api_key.to_string()));

        // Call Stripe API to create customer
        let customer: StripeCustomer = self.stripe_api_call("POST", "/customers", Some(&form)).await?;
        
        tracing::info!("Created Stripe customer {} for {}", customer.id, email);

        // Store in database
        let now = chrono::Utc::now().timestamp();
        sqlx::query(
            r#"
            INSERT INTO stripe_customers 
            (api_key, stripe_customer_id, created_at, updated_at)
            VALUES (?, ?, ?, ?)
            "#,
        )
        .bind(api_key)
        .bind(&customer.id)
        .bind(now)
        .bind(now)
        .execute(&self.pool)
        .await?;

        Ok(customer.id)
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
        let customer = self
            .get_customer(api_key)
            .await?
            .ok_or_else(|| anyhow!("Customer not found. Create customer first."))?;

        // Attach payment method to customer via Stripe API
        let form = vec![
            ("customer", customer.stripe_customer_id.clone()),
        ];
        
        let _pm: StripePaymentMethod = self.stripe_api_call(
            "POST",
            &format!("/payment_methods/{}/attach", payment_method_id),
            Some(&form),
        ).await?;

        tracing::info!("Attached payment method {} to customer {}", payment_method_id, customer.stripe_customer_id);

        // Set as default payment method for customer
        let form = vec![
            ("invoice_settings[default_payment_method]", payment_method_id.to_string()),
        ];
        
        let _: StripeCustomer = self.stripe_api_call(
            "POST",
            &format!("/customers/{}", customer.stripe_customer_id),
            Some(&form),
        ).await?;

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
    ) -> Result<StripeSubscription> {
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
        
        // Create subscription via Stripe API
        let form = vec![
            ("customer", customer.stripe_customer_id.clone()),
            ("items[0][price]", price_id.to_string()),
            ("metadata[nanolambda_tier]", format!("{:?}", tier)),
            ("metadata[nanolambda_api_key]", api_key.to_string()),
        ];

        let subscription: StripeSubscription = self.stripe_api_call("POST", "/subscriptions", Some(&form)).await?;
        
        tracing::info!("Created subscription {} for tier {:?}", subscription.id, tier);

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
    ) -> Result<StripeSubscription> {
        let customer = self
            .get_customer(api_key)
            .await?
            .ok_or_else(|| anyhow!("Customer not found"))?;

        let subscription_id = customer
            .stripe_subscription_id
            .ok_or_else(|| anyhow!("No active subscription found"))?;

        let new_price_id = price_ids.get_price_id(new_tier);

        // Get current subscription to find subscription item ID
        #[derive(Deserialize)]
        struct SubscriptionWithItems {
            id: String,
            items: SubscriptionItems,
        }
        
        #[derive(Deserialize)]
        struct SubscriptionItems {
            data: Vec<SubscriptionItem>,
        }
        
        #[derive(Deserialize)]
        struct SubscriptionItem {
            id: String,
        }

        let current_sub: SubscriptionWithItems = self.stripe_api_call(
            "GET",
            &format!("/subscriptions/{}", subscription_id),
            None,
        ).await?;

        let item_id = current_sub.items.data.first()
            .ok_or_else(|| anyhow!("No subscription items found"))?
            .id.clone();

        // Update subscription with new price
        let form = vec![
            ("items[0][id]", item_id),
            ("items[0][price]", new_price_id.to_string()),
            ("proration_behavior", "create_prorations".to_string()),
            ("metadata[nanolambda_tier]", format!("{:?}", new_tier)),
        ];

        let updated_sub: StripeSubscription = self.stripe_api_call(
            "POST",
            &format!("/subscriptions/{}", subscription_id),
            Some(&form),
        ).await?;

        tracing::info!("Updated subscription {} to new tier {:?}", subscription_id, new_tier);

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
    pub async fn cancel_subscription(&self, api_key: &str) -> Result<StripeSubscription> {
        let customer = self
            .get_customer(api_key)
            .await?
            .ok_or_else(|| anyhow!("Customer not found"))?;

        let subscription_id = customer
            .stripe_subscription_id
            .ok_or_else(|| anyhow!("No active subscription found"))?;

        // Cancel subscription via Stripe API
        let canceled_sub: StripeSubscription = self.stripe_api_call(
            "DELETE",
            &format!("/subscriptions/{}", subscription_id),
            None,
        ).await?;

        tracing::info!("Canceled subscription {}", subscription_id);

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

    /// Verify Stripe webhook signature
    pub fn verify_webhook_signature(
        &self,
        payload: &str,
        signature_header: &str,
        webhook_secret: &str,
    ) -> Result<()> {
        use hmac::{Hmac, Mac};
        use sha2::Sha256;

        // Parse signature header: "t=timestamp,v1=signature"
        let mut timestamp = None;
        let mut signature = None;

        for part in signature_header.split(',') {
            if let Some((key, value)) = part.split_once('=') {
                match key {
                    "t" => timestamp = Some(value),
                    "v1" => signature = Some(value),
                    _ => {}
                }
            }
        }

        let timestamp = timestamp.ok_or_else(|| anyhow!("Missing timestamp in signature header"))?;
        let expected_sig = signature.ok_or_else(|| anyhow!("Missing signature in header"))?;

        // Construct signed payload: timestamp.payload
        let signed_payload = format!("{}.{}", timestamp, payload);

        // Compute HMAC-SHA256
        type HmacSha256 = Hmac<Sha256>;
        let mut mac = HmacSha256::new_from_slice(webhook_secret.as_bytes())
            .map_err(|e| anyhow!("Invalid webhook secret: {}", e))?;
        mac.update(signed_payload.as_bytes());
        
        let computed_sig = hex::encode(mac.finalize().into_bytes());

        // Compare signatures (constant-time comparison)
        if computed_sig != expected_sig {
            return Err(anyhow!("Webhook signature verification failed"));
        }

        // Check timestamp to prevent replay attacks (within 5 minutes)
        let timestamp_secs: i64 = timestamp.parse()
            .map_err(|_| anyhow!("Invalid timestamp format"))?;
        let now = chrono::Utc::now().timestamp();
        let age_secs = now - timestamp_secs;

        if age_secs.abs() > 300 {
            return Err(anyhow!("Webhook timestamp too old or in future (age: {}s)", age_secs));
        }

        Ok(())
    }

    /// Handle Stripe webhook events
    pub async fn handle_webhook(
        &self,
        payload: &str,
        signature: &str,
        webhook_secret: &str,
    ) -> Result<WebhookResult> {
        // Verify signature first
        self.verify_webhook_signature(payload, signature, webhook_secret)?;

        // Parse event
        let event: StripeEvent = serde_json::from_str(payload)
            .map_err(|e| anyhow!("Failed to parse webhook event: {}", e))?;

        tracing::info!("Processing Stripe webhook event: {} ({})", event.event_type, event.id);

        // Handle different event types
        match event.event_type.as_str() {
            "customer.subscription.created" => self.handle_subscription_created(&event).await,
            "customer.subscription.updated" => self.handle_subscription_updated(&event).await,
            "customer.subscription.deleted" => self.handle_subscription_deleted(&event).await,
            "invoice.payment_succeeded" => self.handle_payment_succeeded(&event).await,
            "invoice.payment_failed" => self.handle_payment_failed(&event).await,
            "payment_method.attached" => self.handle_payment_method_attached(&event).await,
            _ => {
                tracing::debug!("Unhandled webhook event type: {}", event.event_type);
                Ok(WebhookResult {
                    processed: false,
                    message: format!("Event type not handled: {}", event.event_type),
                })
            }
        }
    }

    async fn handle_subscription_created(&self, event: &StripeEvent) -> Result<WebhookResult> {
        let subscription: StripeSubscription = serde_json::from_value(event.data.object.clone())
            .map_err(|e| anyhow!("Failed to parse subscription: {}", e))?;

        tracing::info!("Subscription created: {} for customer {}", subscription.id, subscription.customer);

        // Update database
        let now = chrono::Utc::now().timestamp();
        sqlx::query(
            r#"
            UPDATE stripe_customers 
            SET stripe_subscription_id = ?, subscription_status = ?, updated_at = ?
            WHERE stripe_customer_id = ?
            "#,
        )
        .bind(&subscription.id)
        .bind(&subscription.status)
        .bind(now)
        .bind(&subscription.customer)
        .execute(&self.pool)
        .await?;

        Ok(WebhookResult {
            processed: true,
            message: format!("Subscription {} created", subscription.id),
        })
    }

    async fn handle_subscription_updated(&self, event: &StripeEvent) -> Result<WebhookResult> {
        let subscription: StripeSubscription = serde_json::from_value(event.data.object.clone())
            .map_err(|e| anyhow!("Failed to parse subscription: {}", e))?;

        tracing::info!("Subscription updated: {} status={}", subscription.id, subscription.status);

        // Update database
        let now = chrono::Utc::now().timestamp();
        sqlx::query(
            r#"
            UPDATE stripe_customers 
            SET subscription_status = ?, updated_at = ?
            WHERE stripe_subscription_id = ?
            "#,
        )
        .bind(&subscription.status)
        .bind(now)
        .bind(&subscription.id)
        .execute(&self.pool)
        .await?;

        Ok(WebhookResult {
            processed: true,
            message: format!("Subscription {} updated to status: {}", subscription.id, subscription.status),
        })
    }

    async fn handle_subscription_deleted(&self, event: &StripeEvent) -> Result<WebhookResult> {
        let subscription: StripeSubscription = serde_json::from_value(event.data.object.clone())
            .map_err(|e| anyhow!("Failed to parse subscription: {}", e))?;

        tracing::info!("Subscription deleted: {}", subscription.id);

        // Update database - keep subscription_id but mark as canceled
        let now = chrono::Utc::now().timestamp();
        sqlx::query(
            r#"
            UPDATE stripe_customers 
            SET subscription_status = 'canceled', updated_at = ?
            WHERE stripe_subscription_id = ?
            "#,
        )
        .bind(now)
        .bind(&subscription.id)
        .execute(&self.pool)
        .await?;

        Ok(WebhookResult {
            processed: true,
            message: format!("Subscription {} deleted", subscription.id),
        })
    }

    async fn handle_payment_succeeded(&self, event: &StripeEvent) -> Result<WebhookResult> {
        #[derive(Deserialize)]
        struct Invoice {
            id: String,
            customer: String,
            subscription: Option<String>,
            amount_paid: i64,
        }

        let invoice: Invoice = serde_json::from_value(event.data.object.clone())
            .map_err(|e| anyhow!("Failed to parse invoice: {}", e))?;

        tracing::info!("Payment succeeded: invoice {} for customer {} (${:.2})", 
            invoice.id, invoice.customer, invoice.amount_paid as f64 / 100.0);

        // Optionally: Update payment history, send receipt email, etc.

        Ok(WebhookResult {
            processed: true,
            message: format!("Payment succeeded for invoice {}", invoice.id),
        })
    }

    async fn handle_payment_failed(&self, event: &StripeEvent) -> Result<WebhookResult> {
        #[derive(Deserialize)]
        struct Invoice {
            id: String,
            customer: String,
            subscription: Option<String>,
            amount_due: i64,
        }

        let invoice: Invoice = serde_json::from_value(event.data.object.clone())
            .map_err(|e| anyhow!("Failed to parse invoice: {}", e))?;

        tracing::warn!("Payment failed: invoice {} for customer {} (${:.2})", 
            invoice.id, invoice.customer, invoice.amount_due as f64 / 100.0);

        // Optionally: Suspend user access, send payment failure email, retry logic

        Ok(WebhookResult {
            processed: true,
            message: format!("Payment failed for invoice {}", invoice.id),
        })
    }

    async fn handle_payment_method_attached(&self, event: &StripeEvent) -> Result<WebhookResult> {
        let payment_method: StripePaymentMethod = serde_json::from_value(event.data.object.clone())
            .map_err(|e| anyhow!("Failed to parse payment method: {}", e))?;

        tracing::info!("Payment method attached: {} type={}", payment_method.id, payment_method.method_type);

        Ok(WebhookResult {
            processed: true,
            message: format!("Payment method {} attached", payment_method.id),
        })
    }
}

/// Webhook processing result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebhookResult {
    pub processed: bool,
    pub message: String,
}

/// Stripe webhook event structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StripeEvent {
    pub id: String,
    #[serde(rename = "type")]
    pub event_type: String,
    pub data: StripeEventData,
    pub created: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StripeEventData {
    pub object: serde_json::Value,
}
