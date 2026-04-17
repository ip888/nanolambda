use anyhow::{Result, anyhow};
use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Mutex;

/// Payment retry attempt record
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PaymentRetryAttempt {
    pub attempt_number: i32,
    pub attempted_at: DateTime<Utc>,
    pub amount_cents: i64,
    pub status: String, // "failed", "succeeded", "pending"
    pub failure_reason: Option<String>,
    pub next_retry_at: Option<DateTime<Utc>>,
}

/// Payment retry configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RetryConfig {
    pub max_attempts: i32,
    pub retry_delays_hours: Vec<i64>, // e.g., [24, 72, 168] for 1 day, 3 days, 7 days
    pub send_dunning_emails: bool,
    pub suspend_after_final_failure: bool,
}

impl Default for RetryConfig {
    fn default() -> Self {
        Self {
            max_attempts: 3,
            retry_delays_hours: vec![24, 72, 168], // 1 day, 3 days, 7 days
            send_dunning_emails: true,
            suspend_after_final_failure: true,
        }
    }
}

/// Complete payment retry status for a customer
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PaymentRetryStatus {
    pub api_key: String,
    pub account_status: String, // "active", "past_due", "suspended"
    pub outstanding_amount_cents: i64,
    pub failed_at: DateTime<Utc>,
    pub current_attempt: i32,
    pub max_attempts: i32,
    pub next_retry_at: Option<DateTime<Utc>>,
    pub retry_history: Vec<PaymentRetryAttempt>,
    pub dunning_sent: bool,
    pub final_notice_sent: bool,
    pub can_retry_now: bool,
}

/// Platform-wide payment retry metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PaymentRetryMetrics {
    pub total_failed_payments: i64,
    pub accounts_in_retry: i64,
    pub accounts_past_due: i64,
    pub accounts_suspended: i64,
    pub total_outstanding_cents: i64,
    pub successful_retries_count: i64,
    pub failed_retries_count: i64,
    pub recovery_rate_percentage: f64,
    pub avg_recovery_days: f64,
}

/// Payment retry event for analytics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PaymentRetryEvent {
    pub api_key: String,
    pub event_type: String, // "payment_failed", "retry_succeeded", "retry_failed", "account_suspended"
    pub amount_cents: i64,
    pub attempt_number: i32,
    pub occurred_at: DateTime<Utc>,
    pub metadata: Option<String>,
}

/// Manages payment retry logic with exponential backoff
pub struct PaymentRetryManager {
    retry_statuses: Arc<Mutex<HashMap<String, PaymentRetryStatus>>>,
    retry_events: Arc<Mutex<Vec<PaymentRetryEvent>>>,
    config: RetryConfig,
}

impl PaymentRetryManager {
    pub async fn new() -> Result<Self> {
        Ok(Self {
            retry_statuses: Arc::new(Mutex::new(HashMap::new())),
            retry_events: Arc::new(Mutex::new(Vec::new())),
            config: RetryConfig::default(),
        })
    }

    pub async fn new_with_config(config: RetryConfig) -> Result<Self> {
        Ok(Self {
            retry_statuses: Arc::new(Mutex::new(HashMap::new())),
            retry_events: Arc::new(Mutex::new(Vec::new())),
            config,
        })
    }

    /// Record a payment failure and initiate retry process
    pub async fn record_payment_failure(
        &self,
        api_key: &str,
        amount_cents: i64,
        failure_reason: &str,
    ) -> Result<PaymentRetryStatus> {
        let mut statuses = self.retry_statuses.lock().await;
        let mut events = self.retry_events.lock().await;

        let now = Utc::now();

        // Get or create retry status
        let mut status = statuses
            .get(api_key)
            .cloned()
            .unwrap_or_else(|| PaymentRetryStatus {
                api_key: api_key.to_string(),
                account_status: "active".to_string(),
                outstanding_amount_cents: 0,
                failed_at: now,
                current_attempt: 0,
                max_attempts: self.config.max_attempts,
                next_retry_at: None,
                retry_history: Vec::new(),
                dunning_sent: false,
                final_notice_sent: false,
                can_retry_now: false,
            });

        // Update status
        status.outstanding_amount_cents += amount_cents;
        status.current_attempt += 1;
        status.account_status = "past_due".to_string();

        // Calculate next retry time
        let delay_hours = if status.current_attempt <= self.config.retry_delays_hours.len() as i32 {
            self.config.retry_delays_hours[(status.current_attempt - 1) as usize]
        } else {
            *self.config.retry_delays_hours.last().unwrap_or(&168) // Default to 7 days
        };

        let next_retry = if status.current_attempt < self.config.max_attempts {
            Some(now + Duration::hours(delay_hours))
        } else {
            None
        };

        // Add retry attempt to history
        let attempt = PaymentRetryAttempt {
            attempt_number: status.current_attempt,
            attempted_at: now,
            amount_cents,
            status: "failed".to_string(),
            failure_reason: Some(failure_reason.to_string()),
            next_retry_at: next_retry,
        };
        status.retry_history.push(attempt);
        status.next_retry_at = next_retry;

        // Check if final attempt failed
        if status.current_attempt >= self.config.max_attempts {
            if self.config.suspend_after_final_failure {
                status.account_status = "suspended".to_string();
            }
            status.can_retry_now = false;
        } else {
            status.can_retry_now = false; // Will be true after delay period
        }

        // Record event
        events.push(PaymentRetryEvent {
            api_key: api_key.to_string(),
            event_type: "payment_failed".to_string(),
            amount_cents,
            attempt_number: status.current_attempt,
            occurred_at: now,
            metadata: Some(failure_reason.to_string()),
        });

        statuses.insert(api_key.to_string(), status.clone());
        Ok(status)
    }

    /// Process a retry attempt (simulates retry logic)
    pub async fn process_retry(&self, api_key: &str) -> Result<PaymentRetryStatus> {
        let mut statuses = self.retry_statuses.lock().await;
        let mut events = self.retry_events.lock().await;

        let mut status = statuses
            .get(api_key)
            .cloned()
            .ok_or_else(|| anyhow!("No retry status found for this API key"))?;

        if status.account_status == "suspended" {
            return Err(anyhow!("Account is suspended, cannot retry"));
        }

        let now = Utc::now();

        // Check if enough time has passed for retry
        if let Some(next_retry) = status.next_retry_at
            && now < next_retry
        {
            return Err(anyhow!(
                "Too soon to retry. Next retry available at: {}",
                next_retry.to_rfc3339()
            ));
        }

        // Simulate retry (in real system, would call payment processor)
        // For demo: 60% success rate on retries
        let retry_succeeded = (status.current_attempt * 20) % 100 < 60;

        if retry_succeeded {
            // Payment succeeded - increment attempt before success processing
            status.current_attempt += 1;

            let attempt = PaymentRetryAttempt {
                attempt_number: status.current_attempt,
                attempted_at: now,
                amount_cents: status.outstanding_amount_cents,
                status: "succeeded".to_string(),
                failure_reason: None,
                next_retry_at: None,
            };
            status.retry_history.push(attempt);

            // Record success event
            events.push(PaymentRetryEvent {
                api_key: api_key.to_string(),
                event_type: "retry_succeeded".to_string(),
                amount_cents: status.outstanding_amount_cents,
                attempt_number: status.current_attempt,
                occurred_at: now,
                metadata: Some("Payment processed successfully".to_string()),
            });

            // Clear retry status
            status.outstanding_amount_cents = 0;
            status.account_status = "active".to_string();
            status.next_retry_at = None;
            status.can_retry_now = false;

            statuses.insert(api_key.to_string(), status.clone());
        } else {
            // Retry failed
            status.current_attempt += 1;

            let delay_hours =
                if status.current_attempt < self.config.retry_delays_hours.len() as i32 {
                    self.config.retry_delays_hours[status.current_attempt as usize]
                } else {
                    *self.config.retry_delays_hours.last().unwrap_or(&168)
                };

            let next_retry = if status.current_attempt < self.config.max_attempts {
                Some(now + Duration::hours(delay_hours))
            } else {
                None
            };

            let attempt = PaymentRetryAttempt {
                attempt_number: status.current_attempt,
                attempted_at: now,
                amount_cents: status.outstanding_amount_cents,
                status: "failed".to_string(),
                failure_reason: Some("Payment processor declined".to_string()),
                next_retry_at: next_retry,
            };
            status.retry_history.push(attempt);
            status.next_retry_at = next_retry;

            // Record failure event
            events.push(PaymentRetryEvent {
                api_key: api_key.to_string(),
                event_type: "retry_failed".to_string(),
                amount_cents: status.outstanding_amount_cents,
                attempt_number: status.current_attempt,
                occurred_at: now,
                metadata: Some("Payment processor declined".to_string()),
            });

            // Check if all attempts exhausted
            if status.current_attempt >= self.config.max_attempts
                && self.config.suspend_after_final_failure
            {
                status.account_status = "suspended".to_string();

                events.push(PaymentRetryEvent {
                    api_key: api_key.to_string(),
                    event_type: "account_suspended".to_string(),
                    amount_cents: status.outstanding_amount_cents,
                    attempt_number: status.current_attempt,
                    occurred_at: now,
                    metadata: Some("Max retry attempts exceeded".to_string()),
                });
            }

            statuses.insert(api_key.to_string(), status.clone());
        }

        Ok(status)
    }

    /// Get retry status for a customer
    pub async fn get_retry_status(&self, api_key: &str) -> Result<Option<PaymentRetryStatus>> {
        let statuses = self.retry_statuses.lock().await;
        Ok(statuses.get(api_key).cloned())
    }

    /// Get all customers in retry/past due status
    pub async fn get_past_due_customers(&self) -> Result<Vec<PaymentRetryStatus>> {
        let statuses = self.retry_statuses.lock().await;
        Ok(statuses
            .values()
            .filter(|s| s.account_status == "past_due" || s.account_status == "suspended")
            .cloned()
            .collect())
    }

    /// Get platform-wide payment retry metrics
    pub async fn get_platform_metrics(&self) -> Result<PaymentRetryMetrics> {
        let statuses = self.retry_statuses.lock().await;
        let events = self.retry_events.lock().await;

        let total_failed_payments = events
            .iter()
            .filter(|e| e.event_type == "payment_failed")
            .count() as i64;

        let accounts_in_retry = statuses
            .values()
            .filter(|s| s.account_status == "past_due" && s.current_attempt < s.max_attempts)
            .count() as i64;

        let accounts_past_due = statuses
            .values()
            .filter(|s| s.account_status == "past_due")
            .count() as i64;

        let accounts_suspended = statuses
            .values()
            .filter(|s| s.account_status == "suspended")
            .count() as i64;

        let total_outstanding_cents = statuses
            .values()
            .filter(|s| s.account_status != "active")
            .map(|s| s.outstanding_amount_cents)
            .sum();

        let successful_retries = events
            .iter()
            .filter(|e| e.event_type == "retry_succeeded")
            .count() as i64;

        let failed_retries = events
            .iter()
            .filter(|e| e.event_type == "retry_failed")
            .count() as i64;

        let recovery_rate = if total_failed_payments > 0 {
            (successful_retries as f64 / total_failed_payments as f64) * 100.0
        } else {
            0.0
        };

        // Calculate average recovery days (simplified - would need more data in production)
        let avg_recovery_days = 3.5; // Placeholder

        Ok(PaymentRetryMetrics {
            total_failed_payments,
            accounts_in_retry,
            accounts_past_due,
            accounts_suspended,
            total_outstanding_cents,
            successful_retries_count: successful_retries,
            failed_retries_count: failed_retries,
            recovery_rate_percentage: recovery_rate,
            avg_recovery_days,
        })
    }

    /// Clear retry status (after manual resolution)
    pub async fn clear_retry_status(&self, api_key: &str) -> Result<()> {
        let mut statuses = self.retry_statuses.lock().await;
        let mut events = self.retry_events.lock().await;

        if let Some(status) = statuses.remove(api_key) {
            events.push(PaymentRetryEvent {
                api_key: api_key.to_string(),
                event_type: "manually_resolved".to_string(),
                amount_cents: 0,
                attempt_number: status.current_attempt,
                occurred_at: Utc::now(),
                metadata: Some("Manually cleared by admin".to_string()),
            });
        }

        Ok(())
    }

    /// Send dunning notification (simulated - returns notification details)
    pub async fn send_dunning_notification(&self, api_key: &str) -> Result<String> {
        let mut statuses = self.retry_statuses.lock().await;

        if let Some(mut status) = statuses.get(api_key).cloned() {
            let notification_type = if status.current_attempt >= status.max_attempts - 1 {
                "final_notice"
            } else {
                "payment_reminder"
            };

            if notification_type == "final_notice" {
                status.final_notice_sent = true;
            } else {
                status.dunning_sent = true;
            }

            statuses.insert(api_key.to_string(), status.clone());

            Ok(format!(
                "Dunning notification sent: {} - Amount due: ${:.2}, Next retry: {}",
                notification_type,
                status.outstanding_amount_cents as f64 / 100.0,
                status
                    .next_retry_at
                    .map(|d| d.to_rfc3339())
                    .unwrap_or_else(|| "No retry scheduled".to_string())
            ))
        } else {
            Err(anyhow!("No retry status found for this API key"))
        }
    }

    /// Get retry configuration
    pub fn get_config(&self) -> &RetryConfig {
        &self.config
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_retry_config_default() {
        let config = RetryConfig::default();
        assert_eq!(config.max_attempts, 3);
        assert_eq!(config.retry_delays_hours, vec![24, 72, 168]);
        assert!(config.send_dunning_emails);
        assert!(config.suspend_after_final_failure);
    }

    #[tokio::test]
    async fn test_create_retry_manager() {
        let manager = PaymentRetryManager::new().await;
        assert!(manager.is_ok());

        let manager = manager.unwrap();
        assert_eq!(manager.get_config().max_attempts, 3);
    }

    #[tokio::test]
    async fn test_create_retry_manager_with_custom_config() {
        let custom_config = RetryConfig {
            max_attempts: 5,
            retry_delays_hours: vec![12, 24, 48, 96],
            send_dunning_emails: false,
            suspend_after_final_failure: false,
        };

        let manager = PaymentRetryManager::new_with_config(custom_config.clone()).await;
        assert!(manager.is_ok());

        let manager = manager.unwrap();
        assert_eq!(manager.get_config().max_attempts, 5);
        assert_eq!(manager.get_config().retry_delays_hours.len(), 4);
        assert!(!manager.get_config().send_dunning_emails);
    }

    #[tokio::test]
    async fn test_record_payment_failure() {
        let manager = PaymentRetryManager::new().await.unwrap();
        let api_key = "test_key_001";
        let amount = 2999; // $29.99

        let result = manager
            .record_payment_failure(api_key, amount, "insufficient_funds")
            .await;

        assert!(result.is_ok());
        let status = result.unwrap();

        assert_eq!(status.api_key, api_key);
        assert_eq!(status.outstanding_amount_cents, amount);
        assert_eq!(status.account_status, "past_due");
        assert_eq!(status.current_attempt, 1);
        assert_eq!(status.max_attempts, 3);
        assert!(status.retry_history.len() == 1);
    }

    #[tokio::test]
    async fn test_get_retry_status_not_found() {
        let manager = PaymentRetryManager::new().await.unwrap();
        let result = manager.get_retry_status("nonexistent_key").await;

        assert!(result.is_ok());
        assert!(result.unwrap().is_none());
    }

    #[tokio::test]
    async fn test_get_retry_status_exists() {
        let manager = PaymentRetryManager::new().await.unwrap();
        let api_key = "test_key_002";

        // First create a failure
        manager
            .record_payment_failure(api_key, 1999, "card_declined")
            .await
            .unwrap();

        // Then retrieve it
        let result = manager.get_retry_status(api_key).await;
        assert!(result.is_ok());

        let status = result.unwrap();
        assert!(status.is_some());

        let status = status.unwrap();
        assert_eq!(status.api_key, api_key);
        assert_eq!(status.outstanding_amount_cents, 1999);
    }

    #[tokio::test]
    async fn test_process_retry_success() {
        // Create config with zero delays for testing
        let config = RetryConfig {
            max_attempts: 5,
            retry_delays_hours: vec![0, 0, 0, 0],
            send_dunning_emails: false,
            suspend_after_final_failure: true,
        };
        let manager = PaymentRetryManager::new_with_config(config).await.unwrap();
        let api_key = "test_key_003";

        // Record initial failure
        manager
            .record_payment_failure(api_key, 4999, "payment_failed")
            .await
            .unwrap();

        // Process retry (simulates successful payment)
        let result = manager.process_retry(api_key).await;
        assert!(result.is_ok());

        let status = result.unwrap();
        assert_eq!(status.current_attempt, 2);
    }

    #[tokio::test]
    async fn test_clear_retry_status() {
        let manager = PaymentRetryManager::new().await.unwrap();
        let api_key = "test_key_004";

        // Create failure
        manager
            .record_payment_failure(api_key, 999, "declined")
            .await
            .unwrap();

        // Verify exists
        let status_before = manager.get_retry_status(api_key).await.unwrap();
        assert!(status_before.is_some());

        // Clear it
        let clear_result = manager.clear_retry_status(api_key).await;
        assert!(clear_result.is_ok());

        // Verify removed
        let status_after = manager.get_retry_status(api_key).await.unwrap();
        assert!(status_after.is_none());
    }

    #[tokio::test]
    async fn test_get_past_due_customers() {
        let manager = PaymentRetryManager::new().await.unwrap();

        // Create multiple failures
        manager
            .record_payment_failure("key_001", 1000, "failed")
            .await
            .unwrap();
        manager
            .record_payment_failure("key_002", 2000, "failed")
            .await
            .unwrap();
        manager
            .record_payment_failure("key_003", 3000, "failed")
            .await
            .unwrap();

        let result = manager.get_past_due_customers().await;
        assert!(result.is_ok());

        let customers = result.unwrap();
        assert_eq!(customers.len(), 3);

        // Verify all are past_due
        for customer in customers {
            assert_eq!(customer.account_status, "past_due");
        }
    }

    #[tokio::test]
    async fn test_platform_metrics() {
        let manager = PaymentRetryManager::new().await.unwrap();

        // Create some test data
        manager
            .record_payment_failure("metric_key_001", 5000, "failed")
            .await
            .unwrap();
        manager
            .record_payment_failure("metric_key_002", 3000, "failed")
            .await
            .unwrap();

        let result = manager.get_platform_metrics().await;
        assert!(result.is_ok());

        let metrics = result.unwrap();
        assert!(metrics.total_failed_payments > 0);
        assert!(metrics.accounts_in_retry >= 2);
        assert!(metrics.total_outstanding_cents >= 8000);
    }

    #[tokio::test]
    async fn test_retry_attempt_structure() {
        let attempt = PaymentRetryAttempt {
            attempt_number: 1,
            attempted_at: Utc::now(),
            amount_cents: 2999,
            status: "failed".to_string(),
            failure_reason: Some("card_declined".to_string()),
            next_retry_at: Some(Utc::now() + Duration::hours(24)),
        };

        assert_eq!(attempt.attempt_number, 1);
        assert_eq!(attempt.amount_cents, 2999);
        assert_eq!(attempt.status, "failed");
        assert!(attempt.failure_reason.is_some());
    }

    #[tokio::test]
    async fn test_retry_status_structure() {
        let status = PaymentRetryStatus {
            api_key: "test_key".to_string(),
            account_status: "past_due".to_string(),
            outstanding_amount_cents: 4999,
            failed_at: Utc::now(),
            current_attempt: 1,
            max_attempts: 3,
            next_retry_at: Some(Utc::now() + Duration::hours(24)),
            retry_history: vec![],
            dunning_sent: false,
            final_notice_sent: false,
            can_retry_now: true,
        };

        assert_eq!(status.account_status, "past_due");
        assert_eq!(status.outstanding_amount_cents, 4999);
        assert_eq!(status.current_attempt, 1);
        assert!(status.can_retry_now);
    }

    #[tokio::test]
    async fn test_retry_metrics_structure() {
        let metrics = PaymentRetryMetrics {
            total_failed_payments: 100,
            accounts_in_retry: 25,
            accounts_past_due: 20,
            accounts_suspended: 5,
            total_outstanding_cents: 125000,
            successful_retries_count: 15,
            failed_retries_count: 10,
            recovery_rate_percentage: 60.0,
            avg_recovery_days: 3.5,
        };

        assert_eq!(metrics.total_failed_payments, 100);
        assert_eq!(metrics.accounts_in_retry, 25);
        assert!((metrics.recovery_rate_percentage - 60.0).abs() < 0.01);
    }

    #[tokio::test]
    async fn test_send_dunning_notification() {
        let manager = PaymentRetryManager::new().await.unwrap();
        let api_key = "dunning_test_key";

        // Create failure first
        manager
            .record_payment_failure(api_key, 1999, "payment_failed")
            .await
            .unwrap();

        // Send dunning notification
        let result = manager.send_dunning_notification(api_key).await;
        assert!(result.is_ok());

        let message = result.unwrap();
        assert!(!message.is_empty());
        assert!(
            message.contains(api_key) || message.contains("dunning") || message.contains("payment")
        );
    }

    #[tokio::test]
    async fn test_multiple_retry_attempts() {
        // Create config with zero delays for testing
        let config = RetryConfig {
            max_attempts: 5,
            retry_delays_hours: vec![0, 0, 0, 0],
            send_dunning_emails: false,
            suspend_after_final_failure: true,
        };
        let manager = PaymentRetryManager::new_with_config(config).await.unwrap();
        let api_key = "multi_retry_key";

        // Record initial failure
        let status1 = manager
            .record_payment_failure(api_key, 2999, "card_declined")
            .await
            .unwrap();
        assert_eq!(status1.current_attempt, 1);

        // Process first retry
        let status2 = manager.process_retry(api_key).await.unwrap();
        assert_eq!(status2.current_attempt, 2);

        // Process second retry
        let status3 = manager.process_retry(api_key).await.unwrap();
        assert_eq!(status3.current_attempt, 3);
    }

    #[test]
    fn test_retry_config_custom_values() {
        let config = RetryConfig {
            max_attempts: 4,
            retry_delays_hours: vec![6, 12, 24, 48],
            send_dunning_emails: true,
            suspend_after_final_failure: false,
        };

        assert_eq!(config.max_attempts, 4);
        assert_eq!(config.retry_delays_hours[0], 6);
        assert_eq!(config.retry_delays_hours[3], 48);
        assert!(!config.suspend_after_final_failure);
    }
}
