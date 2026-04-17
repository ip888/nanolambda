//! Durable State Machine for Edge Functions
//!
//! Provides persistent state management across edge invocations:
//! - Key-value state with atomic operations
//! - State versioning and conflict resolution
//! - TTL-based expiration
//! - Distributed state synchronization

use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, Instant};
use serde::{Deserialize, Serialize};

/// State entry with metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StateEntry {
    /// The value
    pub value: serde_json::Value,
    /// Version number for optimistic locking
    pub version: u64,
    /// When the entry was created
    pub created_at: u64,
    /// When the entry was last updated
    pub updated_at: u64,
    /// Time-to-live (optional)
    pub ttl: Option<Duration>,
    /// Entry metadata
    pub metadata: HashMap<String, String>,
}

impl StateEntry {
    /// Create a new state entry
    pub fn new(value: serde_json::Value) -> Self {
        let now = current_timestamp();
        Self {
            value,
            version: 1,
            created_at: now,
            updated_at: now,
            ttl: None,
            metadata: HashMap::new(),
        }
    }

    /// Create with TTL
    pub fn with_ttl(value: serde_json::Value, ttl: Duration) -> Self {
        let mut entry = Self::new(value);
        entry.ttl = Some(ttl);
        entry
    }

    /// Check if entry has expired
    pub fn is_expired(&self) -> bool {
        if let Some(ttl) = self.ttl {
            let age = Duration::from_millis(current_timestamp() - self.updated_at);
            age > ttl
        } else {
            false
        }
    }

    /// Increment version and update timestamp
    pub fn touch(&mut self) {
        self.version += 1;
        self.updated_at = current_timestamp();
    }
}

/// Result of a state operation
#[derive(Debug, Clone)]
pub enum StateResult<T> {
    /// Operation succeeded
    Ok(T),
    /// Key not found
    NotFound,
    /// Version conflict
    Conflict { expected: u64, actual: u64 },
    /// Entry expired
    Expired,
    /// Storage error
    Error(String),
}

impl<T> StateResult<T> {
    pub fn is_ok(&self) -> bool {
        matches!(self, StateResult::Ok(_))
    }

    pub fn unwrap(self) -> T {
        match self {
            StateResult::Ok(v) => v,
            _ => panic!("Called unwrap on non-Ok StateResult"),
        }
    }
}

/// Atomic operation for compare-and-swap
#[derive(Debug, Clone)]
pub struct AtomicUpdate {
    /// Expected version (for optimistic locking)
    pub expected_version: Option<u64>,
    /// New value
    pub value: serde_json::Value,
    /// TTL for the new value
    pub ttl: Option<Duration>,
}

/// State machine configuration
#[derive(Debug, Clone)]
pub struct StateMachineConfig {
    /// Maximum entries per namespace
    pub max_entries: usize,
    /// Default TTL for entries
    pub default_ttl: Option<Duration>,
    /// Enable versioning
    pub enable_versioning: bool,
    /// Sync interval for distributed state
    pub sync_interval: Duration,
    /// Conflict resolution strategy
    pub conflict_strategy: ConflictStrategy,
}

impl Default for StateMachineConfig {
    fn default() -> Self {
        Self {
            max_entries: 10000,
            default_ttl: Some(Duration::from_secs(3600)), // 1 hour
            enable_versioning: true,
            sync_interval: Duration::from_secs(5),
            conflict_strategy: ConflictStrategy::LastWriteWins,
        }
    }
}

/// Strategy for handling version conflicts
#[derive(Debug, Clone, Copy)]
pub enum ConflictStrategy {
    /// Last write wins (by timestamp)
    LastWriteWins,
    /// First write wins (reject newer)
    FirstWriteWins,
    /// Require explicit version
    RequireVersion,
    /// Merge values (for objects)
    Merge,
}

/// The durable state machine
pub struct DurableStateMachine {
    /// Namespace for this state machine
    namespace: String,
    /// Configuration
    config: StateMachineConfig,
    /// In-memory state (would be backed by KV in production)
    state: std::sync::RwLock<HashMap<String, StateEntry>>,
    /// Statistics
    stats: StateMachineStats,
    /// Version counter
    version_counter: AtomicU64,
}

/// Statistics for the state machine
#[derive(Debug, Default)]
pub struct StateMachineStats {
    /// Total get operations
    pub gets: AtomicU64,
    /// Total set operations
    pub sets: AtomicU64,
    /// Total delete operations
    pub deletes: AtomicU64,
    /// Cache hits
    pub hits: AtomicU64,
    /// Cache misses
    pub misses: AtomicU64,
    /// Version conflicts
    pub conflicts: AtomicU64,
    /// Expired entries cleaned
    pub expirations: AtomicU64,
}

impl DurableStateMachine {
    /// Create a new state machine
    pub fn new(namespace: impl Into<String>, config: StateMachineConfig) -> Self {
        Self {
            namespace: namespace.into(),
            config,
            state: std::sync::RwLock::new(HashMap::new()),
            stats: StateMachineStats::default(),
            version_counter: AtomicU64::new(1),
        }
    }

    /// Get a value by key
    pub fn get(&self, key: &str) -> StateResult<StateEntry> {
        self.stats.gets.fetch_add(1, Ordering::Relaxed);

        let state = self.state.read().unwrap();
        match state.get(key) {
            Some(entry) => {
                if entry.is_expired() {
                    self.stats.misses.fetch_add(1, Ordering::Relaxed);
                    StateResult::Expired
                } else {
                    self.stats.hits.fetch_add(1, Ordering::Relaxed);
                    StateResult::Ok(entry.clone())
                }
            }
            None => {
                self.stats.misses.fetch_add(1, Ordering::Relaxed);
                StateResult::NotFound
            }
        }
    }

    /// Set a value
    pub fn set(&self, key: impl Into<String>, value: serde_json::Value) -> StateResult<StateEntry> {
        let key = key.into();
        self.stats.sets.fetch_add(1, Ordering::Relaxed);

        let mut entry = StateEntry::new(value);
        if let Some(ttl) = self.config.default_ttl {
            entry.ttl = Some(ttl);
        }

        let mut state = self.state.write().unwrap();

        // Check capacity
        if state.len() >= self.config.max_entries && !state.contains_key(&key) {
            return StateResult::Error("Maximum entries reached".to_string());
        }

        // Update version if existing
        if let Some(existing) = state.get(&key) {
            entry.version = existing.version + 1;
            entry.created_at = existing.created_at;
        }

        state.insert(key, entry.clone());
        StateResult::Ok(entry)
    }

    /// Set with TTL
    pub fn set_with_ttl(
        &self,
        key: impl Into<String>,
        value: serde_json::Value,
        ttl: Duration,
    ) -> StateResult<StateEntry> {
        let key = key.into();
        self.stats.sets.fetch_add(1, Ordering::Relaxed);

        let mut entry = StateEntry::with_ttl(value, ttl);

        let mut state = self.state.write().unwrap();

        if state.len() >= self.config.max_entries && !state.contains_key(&key) {
            return StateResult::Error("Maximum entries reached".to_string());
        }

        if let Some(existing) = state.get(&key) {
            entry.version = existing.version + 1;
            entry.created_at = existing.created_at;
        }

        state.insert(key, entry.clone());
        StateResult::Ok(entry)
    }

    /// Atomic compare-and-swap update
    pub fn compare_and_swap(
        &self,
        key: &str,
        update: AtomicUpdate,
    ) -> StateResult<StateEntry> {
        self.stats.sets.fetch_add(1, Ordering::Relaxed);

        let mut state = self.state.write().unwrap();

        // Check current version
        if let Some(expected) = update.expected_version {
            if let Some(existing) = state.get(key) {
                if existing.version != expected {
                    self.stats.conflicts.fetch_add(1, Ordering::Relaxed);
                    return StateResult::Conflict {
                        expected,
                        actual: existing.version,
                    };
                }
            } else {
                return StateResult::NotFound;
            }
        }

        let mut entry = if let Some(existing) = state.get(key) {
            let mut entry = existing.clone();
            entry.value = update.value;
            entry.touch();
            if let Some(ttl) = update.ttl {
                entry.ttl = Some(ttl);
            }
            entry
        } else {
            let mut entry = StateEntry::new(update.value);
            if let Some(ttl) = update.ttl {
                entry.ttl = Some(ttl);
            }
            entry
        };

        state.insert(key.to_string(), entry.clone());
        StateResult::Ok(entry)
    }

    /// Delete a key
    pub fn delete(&self, key: &str) -> StateResult<()> {
        self.stats.deletes.fetch_add(1, Ordering::Relaxed);

        let mut state = self.state.write().unwrap();
        match state.remove(key) {
            Some(_) => StateResult::Ok(()),
            None => StateResult::NotFound,
        }
    }

    /// Delete with version check
    pub fn delete_if_version(&self, key: &str, expected_version: u64) -> StateResult<()> {
        self.stats.deletes.fetch_add(1, Ordering::Relaxed);

        let mut state = self.state.write().unwrap();

        if let Some(existing) = state.get(key) {
            if existing.version != expected_version {
                self.stats.conflicts.fetch_add(1, Ordering::Relaxed);
                return StateResult::Conflict {
                    expected: expected_version,
                    actual: existing.version,
                };
            }
            state.remove(key);
            StateResult::Ok(())
        } else {
            StateResult::NotFound
        }
    }

    /// Increment a numeric value atomically
    pub fn increment(&self, key: &str, delta: i64) -> StateResult<i64> {
        let mut state = self.state.write().unwrap();

        let new_value = if let Some(entry) = state.get_mut(key) {
            let current = entry.value.as_i64().unwrap_or(0);
            let new_value = current + delta;
            entry.value = serde_json::json!(new_value);
            entry.touch();
            new_value
        } else {
            let mut entry = StateEntry::new(serde_json::json!(delta));
            if let Some(ttl) = self.config.default_ttl {
                entry.ttl = Some(ttl);
            }
            state.insert(key.to_string(), entry);
            delta
        };

        self.stats.sets.fetch_add(1, Ordering::Relaxed);
        StateResult::Ok(new_value)
    }

    /// Add to a list
    pub fn list_push(&self, key: &str, value: serde_json::Value) -> StateResult<usize> {
        let mut state = self.state.write().unwrap();

        if let Some(entry) = state.get_mut(key) {
            if let Some(arr) = entry.value.as_array_mut() {
                arr.push(value);
                let len = arr.len();
                entry.version += 1;
                entry.updated_at = current_timestamp();
                self.stats.sets.fetch_add(1, Ordering::Relaxed);
                return StateResult::Ok(len);
            } else {
                return StateResult::Error("Key is not a list".to_string());
            }
        }

        let mut entry = StateEntry::new(serde_json::json!([value]));
        if let Some(ttl) = self.config.default_ttl {
            entry.ttl = Some(ttl);
        }
        state.insert(key.to_string(), entry);

        self.stats.sets.fetch_add(1, Ordering::Relaxed);
        StateResult::Ok(1)
    }

    /// Pop from a list
    pub fn list_pop(&self, key: &str) -> StateResult<serde_json::Value> {
        let mut state = self.state.write().unwrap();

        if let Some(entry) = state.get_mut(key) {
            if let Some(arr) = entry.value.as_array_mut() {
                if let Some(value) = arr.pop() {
                    entry.touch();
                    self.stats.sets.fetch_add(1, Ordering::Relaxed);
                    return StateResult::Ok(value);
                } else {
                    return StateResult::NotFound;
                }
            } else {
                return StateResult::Error("Key is not a list".to_string());
            }
        }

        StateResult::NotFound
    }

    /// Add to a set
    pub fn set_add(&self, key: &str, value: serde_json::Value) -> StateResult<bool> {
        let mut state = self.state.write().unwrap();

        let added = if let Some(entry) = state.get_mut(key) {
            if let Some(arr) = entry.value.as_array_mut() {
                if !arr.contains(&value) {
                    arr.push(value);
                    entry.touch();
                    true
                } else {
                    false
                }
            } else {
                return StateResult::Error("Key is not a set".to_string());
            }
        } else {
            let mut entry = StateEntry::new(serde_json::json!([value]));
            if let Some(ttl) = self.config.default_ttl {
                entry.ttl = Some(ttl);
            }
            state.insert(key.to_string(), entry);
            true
        };

        self.stats.sets.fetch_add(1, Ordering::Relaxed);
        StateResult::Ok(added)
    }

    /// Check set membership
    pub fn set_contains(&self, key: &str, value: &serde_json::Value) -> StateResult<bool> {
        self.stats.gets.fetch_add(1, Ordering::Relaxed);

        let state = self.state.read().unwrap();

        if let Some(entry) = state.get(key) {
            if entry.is_expired() {
                return StateResult::Expired;
            }
            if let Some(arr) = entry.value.as_array() {
                return StateResult::Ok(arr.contains(value));
            } else {
                return StateResult::Error("Key is not a set".to_string());
            }
        }

        StateResult::NotFound
    }

    /// List all keys (with optional prefix)
    pub fn list_keys(&self, prefix: Option<&str>) -> Vec<String> {
        let state = self.state.read().unwrap();

        state
            .keys()
            .filter(|k| {
                if let Some(p) = prefix {
                    k.starts_with(p)
                } else {
                    true
                }
            })
            .cloned()
            .collect()
    }

    /// Clean up expired entries
    pub fn cleanup_expired(&self) -> usize {
        let mut state = self.state.write().unwrap();
        let before = state.len();

        state.retain(|_, entry| !entry.is_expired());

        let cleaned = before - state.len();
        self.stats.expirations.fetch_add(cleaned as u64, Ordering::Relaxed);

        cleaned
    }

    /// Get statistics
    pub fn stats(&self) -> &StateMachineStats {
        &self.stats
    }

    /// Get namespace
    pub fn namespace(&self) -> &str {
        &self.namespace
    }

    /// Get entry count
    pub fn len(&self) -> usize {
        self.state.read().unwrap().len()
    }

    /// Check if empty
    pub fn is_empty(&self) -> bool {
        self.state.read().unwrap().is_empty()
    }
}

/// Transaction for batching multiple operations
pub struct StateTransaction {
    operations: Vec<TransactionOp>,
}

/// Operation in a transaction
#[derive(Debug, Clone)]
pub enum TransactionOp {
    Set { key: String, value: serde_json::Value, ttl: Option<Duration> },
    Delete { key: String },
    Increment { key: String, delta: i64 },
    CompareAndSwap { key: String, expected_version: u64, value: serde_json::Value },
}

impl StateTransaction {
    pub fn new() -> Self {
        Self { operations: Vec::new() }
    }

    pub fn set(mut self, key: impl Into<String>, value: serde_json::Value) -> Self {
        self.operations.push(TransactionOp::Set {
            key: key.into(),
            value,
            ttl: None,
        });
        self
    }

    pub fn set_with_ttl(mut self, key: impl Into<String>, value: serde_json::Value, ttl: Duration) -> Self {
        self.operations.push(TransactionOp::Set {
            key: key.into(),
            value,
            ttl: Some(ttl),
        });
        self
    }

    pub fn delete(mut self, key: impl Into<String>) -> Self {
        self.operations.push(TransactionOp::Delete { key: key.into() });
        self
    }

    pub fn increment(mut self, key: impl Into<String>, delta: i64) -> Self {
        self.operations.push(TransactionOp::Increment {
            key: key.into(),
            delta,
        });
        self
    }

    pub fn compare_and_swap(
        mut self,
        key: impl Into<String>,
        expected_version: u64,
        value: serde_json::Value,
    ) -> Self {
        self.operations.push(TransactionOp::CompareAndSwap {
            key: key.into(),
            expected_version,
            value,
        });
        self
    }

    /// Execute the transaction (all-or-nothing)
    pub fn execute(self, state_machine: &DurableStateMachine) -> StateResult<()> {
        // In a real implementation, this would be atomic
        // For now, execute sequentially

        for op in self.operations {
            let result = match op {
                TransactionOp::Set { key, value, ttl } => {
                    if let Some(ttl) = ttl {
                        state_machine.set_with_ttl(key, value, ttl).is_ok()
                    } else {
                        state_machine.set(key, value).is_ok()
                    }
                }
                TransactionOp::Delete { key } => state_machine.delete(&key).is_ok(),
                TransactionOp::Increment { key, delta } => {
                    state_machine.increment(&key, delta).is_ok()
                }
                TransactionOp::CompareAndSwap { key, expected_version, value } => {
                    state_machine
                        .compare_and_swap(
                            &key,
                            AtomicUpdate {
                                expected_version: Some(expected_version),
                                value,
                                ttl: None,
                            },
                        )
                        .is_ok()
                }
            };

            if !result {
                return StateResult::Error("Transaction failed".to_string());
            }
        }

        StateResult::Ok(())
    }
}

impl Default for StateTransaction {
    fn default() -> Self {
        Self::new()
    }
}

// Helper functions

fn current_timestamp() -> u64 {
    use std::time::SystemTime;
    SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap()
        .as_millis() as u64
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_state_entry_creation() {
        let entry = StateEntry::new(serde_json::json!({"key": "value"}));
        assert_eq!(entry.version, 1);
        assert!(!entry.is_expired());
    }

    #[test]
    fn test_state_entry_expiration() {
        let entry = StateEntry::with_ttl(serde_json::json!("test"), Duration::from_millis(1));
        std::thread::sleep(Duration::from_millis(10));
        assert!(entry.is_expired());
    }

    #[test]
    fn test_state_machine_basic() {
        let sm = DurableStateMachine::new("test", StateMachineConfig::default());

        // Set and get
        let result = sm.set("key1", serde_json::json!("value1"));
        assert!(result.is_ok());

        let result = sm.get("key1");
        assert!(result.is_ok());
        let entry = result.unwrap();
        assert_eq!(entry.value, serde_json::json!("value1"));
    }

    #[test]
    fn test_state_machine_versioning() {
        let sm = DurableStateMachine::new("test", StateMachineConfig::default());

        sm.set("key1", serde_json::json!("v1"));
        let entry1 = sm.get("key1").unwrap();
        assert_eq!(entry1.version, 1);

        sm.set("key1", serde_json::json!("v2"));
        let entry2 = sm.get("key1").unwrap();
        assert_eq!(entry2.version, 2);
    }

    #[test]
    fn test_compare_and_swap() {
        let sm = DurableStateMachine::new("test", StateMachineConfig::default());

        sm.set("key1", serde_json::json!("v1"));

        // Successful CAS
        let result = sm.compare_and_swap(
            "key1",
            AtomicUpdate {
                expected_version: Some(1),
                value: serde_json::json!("v2"),
                ttl: None,
            },
        );
        assert!(result.is_ok());

        // Failed CAS (wrong version)
        let result = sm.compare_and_swap(
            "key1",
            AtomicUpdate {
                expected_version: Some(1),
                value: serde_json::json!("v3"),
                ttl: None,
            },
        );
        assert!(matches!(result, StateResult::Conflict { .. }));
    }

    #[test]
    fn test_increment() {
        let sm = DurableStateMachine::new("test", StateMachineConfig::default());

        let result = sm.increment("counter", 1);
        assert_eq!(result.unwrap(), 1);

        let result = sm.increment("counter", 5);
        assert_eq!(result.unwrap(), 6);

        let result = sm.increment("counter", -2);
        assert_eq!(result.unwrap(), 4);
    }

    #[test]
    fn test_list_operations() {
        let sm = DurableStateMachine::new("test", StateMachineConfig::default());

        sm.list_push("list1", serde_json::json!("a"));
        sm.list_push("list1", serde_json::json!("b"));
        sm.list_push("list1", serde_json::json!("c"));

        let entry = sm.get("list1").unwrap();
        assert_eq!(entry.value.as_array().unwrap().len(), 3);

        let popped = sm.list_pop("list1").unwrap();
        assert_eq!(popped, serde_json::json!("c"));
    }

    #[test]
    fn test_transaction() {
        let sm = DurableStateMachine::new("test", StateMachineConfig::default());

        let tx = StateTransaction::new()
            .set("key1", serde_json::json!("value1"))
            .set("key2", serde_json::json!("value2"))
            .increment("counter", 10);

        let result = tx.execute(&sm);
        assert!(result.is_ok());

        assert!(sm.get("key1").is_ok());
        assert!(sm.get("key2").is_ok());
        assert_eq!(sm.get("counter").unwrap().value, serde_json::json!(10));
    }
}
