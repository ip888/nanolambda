// Discount code (coupon) management module
use crate::error::{Result, StorageError};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use sqlx::{Row, SqlitePool};
use std::str::FromStr;

const STRIPE_API_BASE: &str = "https://api.stripe.com/v1";

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum DiscountType {
    Percentage,
    Fixed,
}

impl DiscountType {
    pub fn as_str(&self) -> &str {
        match self {
            DiscountType::Percentage => "percentage",
            DiscountType::Fixed => "fixed",
        }
    }
}

impl FromStr for DiscountType {
    type Err = ();

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        Ok(match s.to_lowercase().as_str() {
            "percentage" => DiscountType::Percentage,
            "fixed" => DiscountType::Fixed,
            _ => DiscountType::Percentage,
        })
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiscountCode {
    pub id: i64,
    pub code: String,
    pub discount_type: DiscountType,
    pub amount: i64, // percentage (0-100) or cents for fixed
    pub description: String,
    pub max_uses: Option<i64>, // None = unlimited
    pub current_uses: i64,
    pub expires_at: Option<i64>, // Unix timestamp, None = never expires
    pub stripe_coupon_id: Option<String>,
    pub active: bool,
    pub created_at: i64,
    pub updated_at: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiscountUsage {
    pub id: i64,
    pub discount_code_id: i64,
    pub api_key: String,
    pub applied_amount: i64, // actual discount in cents
    pub original_amount: i64,
    pub final_amount: i64,
    pub used_at: i64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct DiscountValidation {
    pub valid: bool,
    pub discount_code: Option<DiscountCode>,
    pub error_message: Option<String>,
    pub calculated_discount: i64, // in cents
}

pub struct DiscountManager {
    pool: SqlitePool,
    stripe_api_key: String,
}

impl DiscountManager {
    /// Create a new DiscountManager
    pub async fn new(stripe_api_key: String, pool: SqlitePool) -> Result<Self> {
        // Create discount_codes table
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS discount_codes (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                code TEXT NOT NULL UNIQUE COLLATE NOCASE,
                discount_type TEXT NOT NULL,
                amount INTEGER NOT NULL,
                description TEXT NOT NULL,
                max_uses INTEGER,
                current_uses INTEGER NOT NULL DEFAULT 0,
                expires_at INTEGER,
                stripe_coupon_id TEXT,
                active INTEGER NOT NULL DEFAULT 1,
                created_at INTEGER NOT NULL,
                updated_at INTEGER NOT NULL
            )
            "#,
        )
        .execute(&pool)
        .await
        .map_err(|e| StorageError::DatabaseError(e.to_string()))?;

        // Create discount_usage table
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS discount_usage (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                discount_code_id INTEGER NOT NULL,
                api_key TEXT NOT NULL,
                applied_amount INTEGER NOT NULL,
                original_amount INTEGER NOT NULL,
                final_amount INTEGER NOT NULL,
                used_at INTEGER NOT NULL,
                FOREIGN KEY (discount_code_id) REFERENCES discount_codes(id) ON DELETE CASCADE
            )
            "#,
        )
        .execute(&pool)
        .await
        .map_err(|e| StorageError::DatabaseError(e.to_string()))?;

        // Create indices
        sqlx::query(
            r#"
            CREATE INDEX IF NOT EXISTS idx_discount_codes_code 
            ON discount_codes(code)
            "#,
        )
        .execute(&pool)
        .await
        .map_err(|e| StorageError::DatabaseError(e.to_string()))?;

        sqlx::query(
            r#"
            CREATE INDEX IF NOT EXISTS idx_discount_codes_active 
            ON discount_codes(active)
            "#,
        )
        .execute(&pool)
        .await
        .map_err(|e| StorageError::DatabaseError(e.to_string()))?;

        sqlx::query(
            r#"
            CREATE INDEX IF NOT EXISTS idx_discount_usage_code_id 
            ON discount_usage(discount_code_id)
            "#,
        )
        .execute(&pool)
        .await
        .map_err(|e| StorageError::DatabaseError(e.to_string()))?;

        sqlx::query(
            r#"
            CREATE INDEX IF NOT EXISTS idx_discount_usage_api_key 
            ON discount_usage(api_key)
            "#,
        )
        .execute(&pool)
        .await
        .map_err(|e| StorageError::DatabaseError(e.to_string()))?;

        Ok(Self {
            pool,
            stripe_api_key,
        })
    }

    /// Create a new discount code
    pub async fn create_discount(
        &self,
        code: String,
        discount_type: DiscountType,
        amount: i64,
        description: String,
        max_uses: Option<i64>,
        expires_at: Option<i64>,
        stripe_coupon_id: Option<String>,
    ) -> Result<DiscountCode> {
        let now = Utc::now().timestamp();
        let code_upper = code.to_uppercase();

        // Validate amount
        if discount_type == DiscountType::Percentage && !(1..=100).contains(&amount) {
            return Err(StorageError::InvalidConfig(
                "Percentage discount must be between 1 and 100".to_string(),
            ));
        }

        if discount_type == DiscountType::Fixed && amount < 1 {
            return Err(StorageError::InvalidConfig(
                "Fixed discount must be positive".to_string(),
            ));
        }

        let id = sqlx::query(
            r#"
            INSERT INTO discount_codes (
                code, discount_type, amount, description, 
                max_uses, current_uses, expires_at, stripe_coupon_id, 
                active, created_at, updated_at
            )
            VALUES (?, ?, ?, ?, ?, 0, ?, ?, 1, ?, ?)
            "#,
        )
        .bind(&code_upper)
        .bind(discount_type.as_str())
        .bind(amount)
        .bind(&description)
        .bind(max_uses)
        .bind(expires_at)
        .bind(&stripe_coupon_id)
        .bind(now)
        .bind(now)
        .execute(&self.pool)
        .await
        .map_err(|e| StorageError::DatabaseError(e.to_string()))?
        .last_insert_rowid();

        Ok(DiscountCode {
            id,
            code: code_upper,
            discount_type,
            amount,
            description,
            max_uses,
            current_uses: 0,
            expires_at,
            stripe_coupon_id,
            active: true,
            created_at: now,
            updated_at: now,
        })
    }

    /// Get discount code by code string
    pub async fn get_discount_by_code(&self, code: &str) -> Result<Option<DiscountCode>> {
        let code_upper = code.to_uppercase();

        let row = sqlx::query(
            r#"
            SELECT id, code, discount_type, amount, description, 
                   max_uses, current_uses, expires_at, stripe_coupon_id, 
                   active, created_at, updated_at
            FROM discount_codes
            WHERE code = ? COLLATE NOCASE
            "#,
        )
        .bind(&code_upper)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| StorageError::DatabaseError(e.to_string()))?;

        Ok(row.map(|r| DiscountCode {
            id: r.get("id"),
            code: r.get("code"),
            discount_type: DiscountType::from_str(r.get("discount_type")).unwrap(),
            amount: r.get("amount"),
            description: r.get("description"),
            max_uses: r.get("max_uses"),
            current_uses: r.get("current_uses"),
            expires_at: r.get("expires_at"),
            stripe_coupon_id: r.get("stripe_coupon_id"),
            active: r.get::<i64, _>("active") == 1,
            created_at: r.get("created_at"),
            updated_at: r.get("updated_at"),
        }))
    }

    /// Get discount code by ID
    pub async fn get_discount_by_id(&self, id: i64) -> Result<Option<DiscountCode>> {
        let row = sqlx::query(
            r#"
            SELECT id, code, discount_type, amount, description, 
                   max_uses, current_uses, expires_at, stripe_coupon_id, 
                   active, created_at, updated_at
            FROM discount_codes
            WHERE id = ?
            "#,
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| StorageError::DatabaseError(e.to_string()))?;

        Ok(row.map(|r| DiscountCode {
            id: r.get("id"),
            code: r.get("code"),
            discount_type: DiscountType::from_str(r.get("discount_type")).unwrap(),
            amount: r.get("amount"),
            description: r.get("description"),
            max_uses: r.get("max_uses"),
            current_uses: r.get("current_uses"),
            expires_at: r.get("expires_at"),
            stripe_coupon_id: r.get("stripe_coupon_id"),
            active: r.get::<i64, _>("active") == 1,
            created_at: r.get("created_at"),
            updated_at: r.get("updated_at"),
        }))
    }

    /// List all discount codes
    pub async fn list_discounts(&self, active_only: bool) -> Result<Vec<DiscountCode>> {
        let query = if active_only {
            "SELECT * FROM discount_codes WHERE active = 1 ORDER BY created_at DESC"
        } else {
            "SELECT * FROM discount_codes ORDER BY created_at DESC"
        };

        let rows = sqlx::query(query)
            .fetch_all(&self.pool)
            .await
            .map_err(|e| StorageError::DatabaseError(e.to_string()))?;

        Ok(rows
            .into_iter()
            .map(|r| DiscountCode {
                id: r.get("id"),
                code: r.get("code"),
                discount_type: DiscountType::from_str(r.get("discount_type")).unwrap(),
                amount: r.get("amount"),
                description: r.get("description"),
                max_uses: r.get("max_uses"),
                current_uses: r.get("current_uses"),
                expires_at: r.get("expires_at"),
                stripe_coupon_id: r.get("stripe_coupon_id"),
                active: r.get::<i64, _>("active") == 1,
                created_at: r.get("created_at"),
                updated_at: r.get("updated_at"),
            })
            .collect())
    }

    /// Validate a discount code for use
    pub async fn validate_discount(&self, code: &str, amount: i64) -> Result<DiscountValidation> {
        let discount = match self.get_discount_by_code(code).await? {
            Some(d) => d,
            None => {
                return Ok(DiscountValidation {
                    valid: false,
                    discount_code: None,
                    error_message: Some("Discount code not found".to_string()),
                    calculated_discount: 0,
                });
            }
        };

        // Check if active
        if !discount.active {
            return Ok(DiscountValidation {
                valid: false,
                discount_code: Some(discount),
                error_message: Some("Discount code is no longer active".to_string()),
                calculated_discount: 0,
            });
        }

        // Check if expired
        if let Some(expires_at) = discount.expires_at
            && Utc::now().timestamp() > expires_at
        {
            return Ok(DiscountValidation {
                valid: false,
                discount_code: Some(discount),
                error_message: Some("Discount code has expired".to_string()),
                calculated_discount: 0,
            });
        }

        // Check usage limits
        if let Some(max_uses) = discount.max_uses
            && discount.current_uses >= max_uses
        {
            return Ok(DiscountValidation {
                valid: false,
                discount_code: Some(discount),
                error_message: Some("Discount code has reached maximum usage".to_string()),
                calculated_discount: 0,
            });
        }

        // Calculate discount amount
        let calculated_discount = match discount.discount_type {
            DiscountType::Percentage => (amount * discount.amount) / 100,
            DiscountType::Fixed => {
                discount.amount.min(amount) // Can't discount more than the amount
            }
        };

        Ok(DiscountValidation {
            valid: true,
            discount_code: Some(discount),
            error_message: None,
            calculated_discount,
        })
    }

    /// Apply a discount code (record usage and increment counter)
    pub async fn apply_discount(
        &self,
        code: &str,
        api_key: &str,
        original_amount: i64,
    ) -> Result<DiscountUsage> {
        // Validate first
        let validation = self.validate_discount(code, original_amount).await?;

        if !validation.valid {
            return Err(StorageError::InvalidConfig(
                validation
                    .error_message
                    .unwrap_or_else(|| "Invalid discount code".to_string()),
            ));
        }

        let discount = validation.discount_code.unwrap();
        let applied_amount = validation.calculated_discount;
        let final_amount = original_amount - applied_amount;

        // Record usage
        let now = Utc::now().timestamp();
        let usage_id = sqlx::query(
            r#"
            INSERT INTO discount_usage (
                discount_code_id, api_key, applied_amount, 
                original_amount, final_amount, used_at
            )
            VALUES (?, ?, ?, ?, ?, ?)
            "#,
        )
        .bind(discount.id)
        .bind(api_key)
        .bind(applied_amount)
        .bind(original_amount)
        .bind(final_amount)
        .bind(now)
        .execute(&self.pool)
        .await
        .map_err(|e| StorageError::DatabaseError(e.to_string()))?
        .last_insert_rowid();

        // Increment usage counter
        sqlx::query(
            r#"
            UPDATE discount_codes 
            SET current_uses = current_uses + 1, updated_at = ?
            WHERE id = ?
            "#,
        )
        .bind(now)
        .bind(discount.id)
        .execute(&self.pool)
        .await
        .map_err(|e| StorageError::DatabaseError(e.to_string()))?;

        Ok(DiscountUsage {
            id: usage_id,
            discount_code_id: discount.id,
            api_key: api_key.to_string(),
            applied_amount,
            original_amount,
            final_amount,
            used_at: now,
        })
    }

    /// Get usage history for a discount code
    pub async fn get_discount_usage(&self, discount_code_id: i64) -> Result<Vec<DiscountUsage>> {
        let rows = sqlx::query(
            r#"
            SELECT id, discount_code_id, api_key, applied_amount, 
                   original_amount, final_amount, used_at
            FROM discount_usage
            WHERE discount_code_id = ?
            ORDER BY used_at DESC
            "#,
        )
        .bind(discount_code_id)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| StorageError::DatabaseError(e.to_string()))?;

        Ok(rows
            .into_iter()
            .map(|r| DiscountUsage {
                id: r.get("id"),
                discount_code_id: r.get("discount_code_id"),
                api_key: r.get("api_key"),
                applied_amount: r.get("applied_amount"),
                original_amount: r.get("original_amount"),
                final_amount: r.get("final_amount"),
                used_at: r.get("used_at"),
            })
            .collect())
    }

    /// Get usage history for a user
    pub async fn get_user_discount_usage(&self, api_key: &str) -> Result<Vec<DiscountUsage>> {
        let rows = sqlx::query(
            r#"
            SELECT id, discount_code_id, api_key, applied_amount, 
                   original_amount, final_amount, used_at
            FROM discount_usage
            WHERE api_key = ?
            ORDER BY used_at DESC
            "#,
        )
        .bind(api_key)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| StorageError::DatabaseError(e.to_string()))?;

        Ok(rows
            .into_iter()
            .map(|r| DiscountUsage {
                id: r.get("id"),
                discount_code_id: r.get("discount_code_id"),
                api_key: r.get("api_key"),
                applied_amount: r.get("applied_amount"),
                original_amount: r.get("original_amount"),
                final_amount: r.get("final_amount"),
                used_at: r.get("used_at"),
            })
            .collect())
    }

    /// Deactivate a discount code
    pub async fn deactivate_discount(&self, code: &str) -> Result<()> {
        let now = Utc::now().timestamp();

        sqlx::query(
            r#"
            UPDATE discount_codes 
            SET active = 0, updated_at = ?
            WHERE code = ? COLLATE NOCASE
            "#,
        )
        .bind(now)
        .bind(code.to_uppercase())
        .execute(&self.pool)
        .await
        .map_err(|e| StorageError::DatabaseError(e.to_string()))?;

        Ok(())
    }

    /// Reactivate a discount code
    pub async fn reactivate_discount(&self, code: &str) -> Result<()> {
        let now = Utc::now().timestamp();

        sqlx::query(
            r#"
            UPDATE discount_codes 
            SET active = 1, updated_at = ?
            WHERE code = ? COLLATE NOCASE
            "#,
        )
        .bind(now)
        .bind(code.to_uppercase())
        .execute(&self.pool)
        .await
        .map_err(|e| StorageError::DatabaseError(e.to_string()))?;

        Ok(())
    }

    /// Create a Stripe coupon for this discount
    pub async fn create_stripe_coupon(&self, discount_code_id: i64) -> Result<String> {
        let discount = self
            .get_discount_by_id(discount_code_id)
            .await?
            .ok_or_else(|| StorageError::NotFound)?;

        let client = reqwest::Client::new();

        // Build form data
        let mut form_data = vec![("name", discount.code.clone())];

        match discount.discount_type {
            DiscountType::Percentage => {
                form_data.push(("percent_off", discount.amount.to_string()));
            }
            DiscountType::Fixed => {
                form_data.push(("amount_off", discount.amount.to_string()));
                form_data.push(("currency", "usd".to_string()));
            }
        }

        if let Some(max_redemptions) = discount.max_uses {
            form_data.push(("max_redemptions", max_redemptions.to_string()));
        }

        if let Some(expires_at) = discount.expires_at {
            form_data.push(("redeem_by", expires_at.to_string()));
        }

        let response = client
            .post(format!("{}/coupons", STRIPE_API_BASE))
            .basic_auth(&self.stripe_api_key, Some(""))
            .form(&form_data)
            .send()
            .await
            .map_err(|e| StorageError::ExternalApiError(e.to_string()))?;

        if !response.status().is_success() {
            let error_text = response
                .text()
                .await
                .unwrap_or_else(|_| "Unknown error".to_string());
            return Err(StorageError::ExternalApiError(format!(
                "Stripe API error: {}",
                error_text
            )));
        }

        let stripe_coupon: serde_json::Value = response
            .json()
            .await
            .map_err(|e| StorageError::ExternalApiError(e.to_string()))?;

        let coupon_id = stripe_coupon
            .get("id")
            .and_then(|id| id.as_str())
            .ok_or_else(|| StorageError::ExternalApiError("No coupon ID in response".to_string()))?
            .to_string();

        // Update discount code with Stripe ID
        let now = Utc::now().timestamp();
        sqlx::query(
            r#"
            UPDATE discount_codes 
            SET stripe_coupon_id = ?, updated_at = ?
            WHERE id = ?
            "#,
        )
        .bind(&coupon_id)
        .bind(now)
        .bind(discount_code_id)
        .execute(&self.pool)
        .await
        .map_err(|e| StorageError::DatabaseError(e.to_string()))?;

        Ok(coupon_id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_percentage_discount_calculation() {
        let pool = sqlx::sqlite::SqlitePoolOptions::new()
            .max_connections(1)
            .connect("sqlite::memory:")
            .await
            .unwrap();

        let mgr = DiscountManager::new("sk_test_key".to_string(), pool)
            .await
            .unwrap();

        // Create 20% discount
        let discount = mgr
            .create_discount(
                "SAVE20".to_string(),
                DiscountType::Percentage,
                20,
                "20% off".to_string(),
                None,
                None,
                None,
            )
            .await
            .unwrap();

        // Validate discount on $100 order
        let validation = mgr.validate_discount(&discount.code, 10000).await.unwrap();

        assert!(validation.valid);
        assert_eq!(validation.calculated_discount, 2000); // $20
    }

    #[tokio::test]
    async fn test_fixed_discount_calculation() {
        let pool = sqlx::sqlite::SqlitePoolOptions::new()
            .max_connections(1)
            .connect("sqlite::memory:")
            .await
            .unwrap();

        let mgr = DiscountManager::new("sk_test_key".to_string(), pool)
            .await
            .unwrap();

        // Create $10 discount
        let discount = mgr
            .create_discount(
                "SAVE10".to_string(),
                DiscountType::Fixed,
                1000,
                "$10 off".to_string(),
                None,
                None,
                None,
            )
            .await
            .unwrap();

        // Validate discount on $100 order
        let validation = mgr.validate_discount(&discount.code, 10000).await.unwrap();

        assert!(validation.valid);
        assert_eq!(validation.calculated_discount, 1000); // $10
    }

    #[tokio::test]
    async fn test_expired_discount() {
        let pool = sqlx::sqlite::SqlitePoolOptions::new()
            .max_connections(1)
            .connect("sqlite::memory:")
            .await
            .unwrap();

        let mgr = DiscountManager::new("sk_test_key".to_string(), pool)
            .await
            .unwrap();

        // Create expired discount (expired yesterday)
        let yesterday = Utc::now().timestamp() - (24 * 60 * 60);
        let discount = mgr
            .create_discount(
                "EXPIRED".to_string(),
                DiscountType::Percentage,
                50,
                "Expired discount".to_string(),
                None,
                Some(yesterday),
                None,
            )
            .await
            .unwrap();

        // Should be invalid
        let validation = mgr.validate_discount(&discount.code, 10000).await.unwrap();

        assert!(!validation.valid);
        assert!(validation.error_message.unwrap().contains("expired"));
    }
}
