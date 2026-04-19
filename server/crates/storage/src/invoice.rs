// Invoice generation and storage module
use crate::error::{Result, StorageError};
use crate::tier::TierLevel;
use chrono::Utc;
use serde::{Deserialize, Serialize};
use sqlx::{Row, SqlitePool};
use std::str::FromStr;

const STRIPE_API_BASE: &str = "https://api.stripe.com/v1";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Invoice {
    pub id: i64,
    pub api_key: String,
    pub stripe_invoice_id: Option<String>,
    pub invoice_number: String,
    pub amount_due: i64,  // in cents
    pub amount_paid: i64, // in cents
    pub currency: String,
    pub status: InvoiceStatus,
    pub period_start: i64,
    pub period_end: i64,
    pub due_date: Option<i64>,
    pub paid_at: Option<i64>,
    pub tier_level: TierLevel,
    pub description: String,
    pub invoice_pdf_url: Option<String>,
    pub hosted_invoice_url: Option<String>,
    pub created_at: i64,
    pub updated_at: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, sqlx::Type)]
#[sqlx(rename_all = "lowercase")]
pub enum InvoiceStatus {
    Draft,
    Open,
    Paid,
    Void,
    Uncollectible,
}

impl InvoiceStatus {
    pub fn as_str(&self) -> &str {
        match self {
            InvoiceStatus::Draft => "draft",
            InvoiceStatus::Open => "open",
            InvoiceStatus::Paid => "paid",
            InvoiceStatus::Void => "void",
            InvoiceStatus::Uncollectible => "uncollectible",
        }
    }
}

impl FromStr for InvoiceStatus {
    type Err = ();

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        Ok(match s.to_lowercase().as_str() {
            "draft" => InvoiceStatus::Draft,
            "open" => InvoiceStatus::Open,
            "paid" => InvoiceStatus::Paid,
            "void" => InvoiceStatus::Void,
            "uncollectible" => InvoiceStatus::Uncollectible,
            _ => InvoiceStatus::Draft,
        })
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InvoiceLineItem {
    pub id: i64,
    pub invoice_id: i64,
    pub description: String,
    pub quantity: i64,
    pub unit_amount: i64, // in cents
    pub amount: i64,      // in cents
    pub currency: String,
    pub period_start: i64,
    pub period_end: i64,
}

#[derive(Debug, Serialize)]
pub struct InvoiceSummary {
    pub total_invoices: i64,
    pub total_paid: i64,
    pub total_outstanding: i64,
    pub recent_invoices: Vec<Invoice>,
}

pub struct InvoiceManager {
    pool: SqlitePool,
    stripe_api_key: String,
}

impl InvoiceManager {
    /// Create a new InvoiceManager
    pub async fn new(stripe_api_key: String, pool: SqlitePool) -> Result<Self> {
        // Create invoices table
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS invoices (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                api_key TEXT NOT NULL,
                stripe_invoice_id TEXT UNIQUE,
                invoice_number TEXT NOT NULL UNIQUE,
                amount_due INTEGER NOT NULL,
                amount_paid INTEGER NOT NULL DEFAULT 0,
                currency TEXT NOT NULL DEFAULT 'usd',
                status TEXT NOT NULL,
                period_start INTEGER NOT NULL,
                period_end INTEGER NOT NULL,
                due_date INTEGER,
                paid_at INTEGER,
                tier_level TEXT NOT NULL,
                description TEXT NOT NULL,
                invoice_pdf_url TEXT,
                hosted_invoice_url TEXT,
                created_at INTEGER NOT NULL,
                updated_at INTEGER NOT NULL,
                FOREIGN KEY (api_key) REFERENCES api_keys(key)
            )
            "#,
        )
        .execute(&pool)
        .await
        .map_err(|e| StorageError::DatabaseError(e.to_string()))?;

        // Create invoice line items table
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS invoice_line_items (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                invoice_id INTEGER NOT NULL,
                description TEXT NOT NULL,
                quantity INTEGER NOT NULL,
                unit_amount INTEGER NOT NULL,
                amount INTEGER NOT NULL,
                currency TEXT NOT NULL DEFAULT 'usd',
                period_start INTEGER NOT NULL,
                period_end INTEGER NOT NULL,
                FOREIGN KEY (invoice_id) REFERENCES invoices(id) ON DELETE CASCADE
            )
            "#,
        )
        .execute(&pool)
        .await
        .map_err(|e| StorageError::DatabaseError(e.to_string()))?;

        // Create indices for efficient queries
        sqlx::query(
            r#"
            CREATE INDEX IF NOT EXISTS idx_invoices_api_key 
            ON invoices(api_key)
            "#,
        )
        .execute(&pool)
        .await
        .map_err(|e| StorageError::DatabaseError(e.to_string()))?;

        sqlx::query(
            r#"
            CREATE INDEX IF NOT EXISTS idx_invoices_stripe_id 
            ON invoices(stripe_invoice_id)
            "#,
        )
        .execute(&pool)
        .await
        .map_err(|e| StorageError::DatabaseError(e.to_string()))?;

        sqlx::query(
            r#"
            CREATE INDEX IF NOT EXISTS idx_invoices_status 
            ON invoices(status)
            "#,
        )
        .execute(&pool)
        .await
        .map_err(|e| StorageError::DatabaseError(e.to_string()))?;

        sqlx::query(
            r#"
            CREATE INDEX IF NOT EXISTS idx_invoice_line_items_invoice_id 
            ON invoice_line_items(invoice_id)
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

    /// Generate a unique invoice number
    fn generate_invoice_number() -> String {
        let now = Utc::now();
        let random_suffix: u32 = rand::random::<u32>() % 10000;
        format!("INV-{}-{:04}", now.format("%Y%m"), random_suffix)
    }

    /// Create a new invoice
    pub async fn create_invoice(
        &self,
        api_key: &str,
        tier_level: TierLevel,
        amount_due: i64,
        period_start: i64,
        period_end: i64,
        description: String,
        stripe_invoice_id: Option<String>,
    ) -> Result<Invoice> {
        let now = Utc::now().timestamp();
        let invoice_number = Self::generate_invoice_number();

        let id = sqlx::query(
            r#"
            INSERT INTO invoices (
                api_key, stripe_invoice_id, invoice_number, amount_due, 
                amount_paid, currency, status, period_start, period_end,
                tier_level, description, created_at, updated_at
            )
            VALUES (?, ?, ?, ?, 0, 'usd', ?, ?, ?, ?, ?, ?, ?)
            "#,
        )
        .bind(api_key)
        .bind(&stripe_invoice_id)
        .bind(&invoice_number)
        .bind(amount_due)
        .bind(InvoiceStatus::Open.as_str())
        .bind(period_start)
        .bind(period_end)
        .bind(tier_level.as_str())
        .bind(&description)
        .bind(now)
        .bind(now)
        .execute(&self.pool)
        .await
        .map_err(|e| StorageError::DatabaseError(e.to_string()))?
        .last_insert_rowid();

        Ok(Invoice {
            id,
            api_key: api_key.to_string(),
            stripe_invoice_id,
            invoice_number,
            amount_due,
            amount_paid: 0,
            currency: "usd".to_string(),
            status: InvoiceStatus::Open,
            period_start,
            period_end,
            due_date: None,
            paid_at: None,
            tier_level,
            description,
            invoice_pdf_url: None,
            hosted_invoice_url: None,
            created_at: now,
            updated_at: now,
        })
    }

    /// Add a line item to an invoice
    pub async fn add_line_item(
        &self,
        invoice_id: i64,
        description: String,
        quantity: i64,
        unit_amount: i64,
        period_start: i64,
        period_end: i64,
    ) -> Result<InvoiceLineItem> {
        let amount = quantity * unit_amount;

        let id = sqlx::query(
            r#"
            INSERT INTO invoice_line_items (
                invoice_id, description, quantity, unit_amount, 
                amount, currency, period_start, period_end
            )
            VALUES (?, ?, ?, ?, ?, 'usd', ?, ?)
            "#,
        )
        .bind(invoice_id)
        .bind(&description)
        .bind(quantity)
        .bind(unit_amount)
        .bind(amount)
        .bind(period_start)
        .bind(period_end)
        .execute(&self.pool)
        .await
        .map_err(|e| StorageError::DatabaseError(e.to_string()))?
        .last_insert_rowid();

        Ok(InvoiceLineItem {
            id,
            invoice_id,
            description,
            quantity,
            unit_amount,
            amount,
            currency: "usd".to_string(),
            period_start,
            period_end,
        })
    }

    /// Get invoice by ID
    pub async fn get_invoice(&self, invoice_id: i64) -> Result<Option<Invoice>> {
        let row = sqlx::query(
            r#"
            SELECT id, api_key, stripe_invoice_id, invoice_number, 
                   amount_due, amount_paid, currency, status, 
                   period_start, period_end, due_date, paid_at,
                   tier_level, description, invoice_pdf_url, 
                   hosted_invoice_url, created_at, updated_at
            FROM invoices
            WHERE id = ?
            "#,
        )
        .bind(invoice_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| StorageError::DatabaseError(e.to_string()))?;

        Ok(row.map(|r| Invoice {
            id: r.get("id"),
            api_key: r.get("api_key"),
            stripe_invoice_id: r.get("stripe_invoice_id"),
            invoice_number: r.get("invoice_number"),
            amount_due: r.get("amount_due"),
            amount_paid: r.get("amount_paid"),
            currency: r.get("currency"),
            status: InvoiceStatus::from_str(r.get("status")).unwrap_or(InvoiceStatus::Draft),
            period_start: r.get("period_start"),
            period_end: r.get("period_end"),
            due_date: r.get("due_date"),
            paid_at: r.get("paid_at"),
            tier_level: TierLevel::from_str(r.get("tier_level"))
                .ok()
                .unwrap_or(TierLevel::Starter),
            description: r.get("description"),
            invoice_pdf_url: r.get("invoice_pdf_url"),
            hosted_invoice_url: r.get("hosted_invoice_url"),
            created_at: r.get("created_at"),
            updated_at: r.get("updated_at"),
        }))
    }

    /// Get invoice by Stripe invoice ID
    pub async fn get_invoice_by_stripe_id(
        &self,
        stripe_invoice_id: &str,
    ) -> Result<Option<Invoice>> {
        let row = sqlx::query(
            r#"
            SELECT id, api_key, stripe_invoice_id, invoice_number, 
                   amount_due, amount_paid, currency, status, 
                   period_start, period_end, due_date, paid_at,
                   tier_level, description, invoice_pdf_url, 
                   hosted_invoice_url, created_at, updated_at
            FROM invoices
            WHERE stripe_invoice_id = ?
            "#,
        )
        .bind(stripe_invoice_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| StorageError::DatabaseError(e.to_string()))?;

        Ok(row.map(|r| Invoice {
            id: r.get("id"),
            api_key: r.get("api_key"),
            stripe_invoice_id: r.get("stripe_invoice_id"),
            invoice_number: r.get("invoice_number"),
            amount_due: r.get("amount_due"),
            amount_paid: r.get("amount_paid"),
            currency: r.get("currency"),
            status: InvoiceStatus::from_str(r.get("status")).unwrap_or(InvoiceStatus::Draft),
            period_start: r.get("period_start"),
            period_end: r.get("period_end"),
            due_date: r.get("due_date"),
            paid_at: r.get("paid_at"),
            tier_level: TierLevel::from_str(r.get("tier_level"))
                .ok()
                .unwrap_or(TierLevel::Starter),
            description: r.get("description"),
            invoice_pdf_url: r.get("invoice_pdf_url"),
            hosted_invoice_url: r.get("hosted_invoice_url"),
            created_at: r.get("created_at"),
            updated_at: r.get("updated_at"),
        }))
    }

    /// Get all invoices for a user
    pub async fn list_invoices(
        &self,
        api_key: &str,
        limit: Option<i64>,
        status_filter: Option<InvoiceStatus>,
    ) -> Result<Vec<Invoice>> {
        let limit = limit.unwrap_or(100);

        let rows = if let Some(status) = status_filter {
            sqlx::query(
                r#"
                SELECT id, api_key, stripe_invoice_id, invoice_number, 
                       amount_due, amount_paid, currency, status, 
                       period_start, period_end, due_date, paid_at,
                       tier_level, description, invoice_pdf_url, 
                       hosted_invoice_url, created_at, updated_at
                FROM invoices
                WHERE api_key = ? AND status = ?
                ORDER BY created_at DESC
                LIMIT ?
                "#,
            )
            .bind(api_key)
            .bind(status.as_str())
            .bind(limit)
            .fetch_all(&self.pool)
            .await
            .map_err(|e| StorageError::DatabaseError(e.to_string()))?
        } else {
            sqlx::query(
                r#"
                SELECT id, api_key, stripe_invoice_id, invoice_number, 
                       amount_due, amount_paid, currency, status, 
                       period_start, period_end, due_date, paid_at,
                       tier_level, description, invoice_pdf_url, 
                       hosted_invoice_url, created_at, updated_at
                FROM invoices
                WHERE api_key = ?
                ORDER BY created_at DESC
                LIMIT ?
                "#,
            )
            .bind(api_key)
            .bind(limit)
            .fetch_all(&self.pool)
            .await
            .map_err(|e| StorageError::DatabaseError(e.to_string()))?
        };

        Ok(rows
            .into_iter()
            .map(|r| Invoice {
                id: r.get("id"),
                api_key: r.get("api_key"),
                stripe_invoice_id: r.get("stripe_invoice_id"),
                invoice_number: r.get("invoice_number"),
                amount_due: r.get("amount_due"),
                amount_paid: r.get("amount_paid"),
                currency: r.get("currency"),
                status: InvoiceStatus::from_str(r.get("status")).unwrap_or(InvoiceStatus::Draft),
                period_start: r.get("period_start"),
                period_end: r.get("period_end"),
                due_date: r.get("due_date"),
                paid_at: r.get("paid_at"),
                tier_level: TierLevel::from_str(r.get("tier_level"))
                    .ok()
                    .unwrap_or(TierLevel::Starter),
                description: r.get("description"),
                invoice_pdf_url: r.get("invoice_pdf_url"),
                hosted_invoice_url: r.get("hosted_invoice_url"),
                created_at: r.get("created_at"),
                updated_at: r.get("updated_at"),
            })
            .collect())
    }

    /// Get line items for an invoice
    pub async fn get_line_items(&self, invoice_id: i64) -> Result<Vec<InvoiceLineItem>> {
        let items = sqlx::query_as::<_, (i64, i64, String, i64, i64, i64, String, i64, i64)>(
            r#"
            SELECT id, invoice_id, description, quantity, unit_amount, 
                   amount, currency, period_start, period_end
            FROM invoice_line_items
            WHERE invoice_id = ?
            ORDER BY id
            "#,
        )
        .bind(invoice_id)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| StorageError::DatabaseError(e.to_string()))?;

        Ok(items
            .into_iter()
            .map(
                |(
                    id,
                    invoice_id,
                    description,
                    quantity,
                    unit_amount,
                    amount,
                    currency,
                    period_start,
                    period_end,
                )| {
                    InvoiceLineItem {
                        id,
                        invoice_id,
                        description,
                        quantity,
                        unit_amount,
                        amount,
                        currency,
                        period_start,
                        period_end,
                    }
                },
            )
            .collect())
    }

    /// Update invoice status (e.g., mark as paid)
    pub async fn update_invoice_status(
        &self,
        invoice_id: i64,
        status: InvoiceStatus,
        amount_paid: Option<i64>,
        paid_at: Option<i64>,
    ) -> Result<()> {
        let now = Utc::now().timestamp();

        if let Some(paid) = amount_paid {
            sqlx::query(
                r#"
                UPDATE invoices 
                SET status = ?, amount_paid = ?, paid_at = ?, updated_at = ?
                WHERE id = ?
                "#,
            )
            .bind(status.as_str())
            .bind(paid)
            .bind(paid_at)
            .bind(now)
            .bind(invoice_id)
            .execute(&self.pool)
            .await
            .map_err(|e| StorageError::DatabaseError(e.to_string()))?;
        } else {
            sqlx::query(
                r#"
                UPDATE invoices 
                SET status = ?, updated_at = ?
                WHERE id = ?
                "#,
            )
            .bind(status.as_str())
            .bind(now)
            .bind(invoice_id)
            .execute(&self.pool)
            .await
            .map_err(|e| StorageError::DatabaseError(e.to_string()))?;
        }

        Ok(())
    }

    /// Update invoice with Stripe URLs
    pub async fn update_invoice_urls(
        &self,
        invoice_id: i64,
        invoice_pdf_url: Option<String>,
        hosted_invoice_url: Option<String>,
    ) -> Result<()> {
        let now = Utc::now().timestamp();

        sqlx::query(
            r#"
            UPDATE invoices 
            SET invoice_pdf_url = ?, hosted_invoice_url = ?, updated_at = ?
            WHERE id = ?
            "#,
        )
        .bind(invoice_pdf_url)
        .bind(hosted_invoice_url)
        .bind(now)
        .bind(invoice_id)
        .execute(&self.pool)
        .await
        .map_err(|e| StorageError::DatabaseError(e.to_string()))?;

        Ok(())
    }

    /// Get invoice summary for a user
    pub async fn get_invoice_summary(&self, api_key: &str) -> Result<InvoiceSummary> {
        let total_invoices: i64 =
            sqlx::query_scalar("SELECT COUNT(*) FROM invoices WHERE api_key = ?")
                .bind(api_key)
                .fetch_one(&self.pool)
                .await
                .map_err(|e| StorageError::DatabaseError(e.to_string()))?;

        let total_paid: i64 = sqlx::query_scalar(
            "SELECT COALESCE(SUM(amount_paid), 0) FROM invoices WHERE api_key = ? AND status = 'paid'"
        )
        .bind(api_key)
        .fetch_one(&self.pool)
        .await
        .map_err(|e| StorageError::DatabaseError(e.to_string()))?;

        let total_outstanding: i64 = sqlx::query_scalar(
            "SELECT COALESCE(SUM(amount_due - amount_paid), 0) FROM invoices WHERE api_key = ? AND status IN ('open', 'draft')"
        )
        .bind(api_key)
        .fetch_one(&self.pool)
        .await
        .map_err(|e| StorageError::DatabaseError(e.to_string()))?;

        let recent_invoices = self.list_invoices(api_key, Some(10), None).await?;

        Ok(InvoiceSummary {
            total_invoices,
            total_paid,
            total_outstanding,
            recent_invoices,
        })
    }

    /// Sync invoice from Stripe (update status, URLs, etc.)
    pub async fn sync_stripe_invoice(&self, stripe_invoice_id: &str) -> Result<Option<Invoice>> {
        // Fetch invoice from Stripe API
        let url = format!("{}/invoices/{}", STRIPE_API_BASE, stripe_invoice_id);

        let client = reqwest::Client::new();
        let response = client
            .get(&url)
            .basic_auth(&self.stripe_api_key, Some(""))
            .send()
            .await
            .map_err(|e| crate::error::StorageError::ExternalApiError(e.to_string()))?;

        if !response.status().is_success() {
            return Err(crate::error::StorageError::ExternalApiError(format!(
                "Stripe API error: {}",
                response.status()
            )));
        }

        let stripe_invoice: serde_json::Value = response
            .json()
            .await
            .map_err(|e| crate::error::StorageError::ExternalApiError(e.to_string()))?;

        // Get our invoice record
        let mut invoice = match self.get_invoice_by_stripe_id(stripe_invoice_id).await? {
            Some(inv) => inv,
            None => return Ok(None),
        };

        // Update status
        if let Some(status_str) = stripe_invoice.get("status").and_then(|s| s.as_str()) {
            invoice.status = InvoiceStatus::from_str(status_str).unwrap_or(InvoiceStatus::Draft);
        }

        // Update payment info
        if let Some(amount_paid) = stripe_invoice.get("amount_paid").and_then(|a| a.as_i64()) {
            invoice.amount_paid = amount_paid;
        }

        if let Some(status_transitions) = stripe_invoice.get("status_transitions")
            && let Some(paid_at) = status_transitions.get("paid_at").and_then(|p| p.as_i64())
        {
            invoice.paid_at = Some(paid_at);
        }

        // Update URLs
        if let Some(pdf_url) = stripe_invoice.get("invoice_pdf").and_then(|u| u.as_str()) {
            invoice.invoice_pdf_url = Some(pdf_url.to_string());
        }

        if let Some(hosted_url) = stripe_invoice
            .get("hosted_invoice_url")
            .and_then(|u| u.as_str())
        {
            invoice.hosted_invoice_url = Some(hosted_url.to_string());
        }

        // Update database
        self.update_invoice_status(
            invoice.id,
            invoice.status.clone(),
            Some(invoice.amount_paid),
            invoice.paid_at,
        )
        .await?;

        self.update_invoice_urls(
            invoice.id,
            invoice.invoice_pdf_url.clone(),
            invoice.hosted_invoice_url.clone(),
        )
        .await?;

        Ok(Some(invoice))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_invoice_number_generation() {
        let number = InvoiceManager::generate_invoice_number();
        assert!(number.starts_with("INV-"));
        assert!(number.len() > 12); // INV-YYYYMM-XXXX
    }
}
