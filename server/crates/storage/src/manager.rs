use crate::error::{Result, StorageError};
use crate::models::*;
use r2d2::Pool;
use r2d2_sqlite::SqliteConnectionManager;
use rusqlite::{OptionalExtension, params};
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::path::Path;
use std::str::FromStr;
use std::time::{SystemTime, UNIX_EPOCH};
use tracing::{debug, info, warn};

type DbPool = Pool<SqliteConnectionManager>;

pub struct StorageManager {
    pool: DbPool,
}

impl StorageManager {
    /// Create a new storage manager with the given database path
    pub fn new<P: AsRef<Path>>(db_path: P) -> Result<Self> {
        let manager = SqliteConnectionManager::file(db_path);
        let pool = Pool::new(manager)?;

        let storage = Self { pool };
        storage.configure_sqlite()?;
        storage.init_schema()?;

        info!("Storage manager initialized");
        Ok(storage)
    }

    /// Create an in-memory database (useful for testing)
    pub fn new_in_memory() -> Result<Self> {
        let manager = SqliteConnectionManager::memory();
        let pool = Pool::new(manager)?;

        let storage = Self { pool };
        storage.configure_sqlite()?;
        storage.init_schema()?;

        Ok(storage)
    }

    /// Configure SQLite for production performance and concurrency
    fn configure_sqlite(&self) -> Result<()> {
        let conn = self.pool.get()?;

        // WAL mode: allows concurrent readers with a single writer
        conn.execute_batch(
            "PRAGMA journal_mode = WAL;
             PRAGMA busy_timeout = 5000;
             PRAGMA synchronous = NORMAL;
             PRAGMA cache_size = -64000;
             PRAGMA foreign_keys = ON;
             PRAGMA temp_store = MEMORY;
             PRAGMA mmap_size = 268435456;",
        )?;

        debug!("SQLite configured: WAL mode, 5s busy timeout, 64MB cache");
        Ok(())
    }

    /// Initialize database schema
    fn init_schema(&self) -> Result<()> {
        let conn = self.pool.get()?;

        conn.execute(
            "CREATE TABLE IF NOT EXISTS functions (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                name TEXT NOT NULL,
                runtime TEXT NOT NULL,
                handler TEXT NOT NULL,
                code TEXT NOT NULL,
                code_hash TEXT NOT NULL,
                memory_mb INTEGER NOT NULL,
                timeout_ms INTEGER NOT NULL,
                environment TEXT NOT NULL,
                created_at INTEGER NOT NULL,
                updated_at INTEGER NOT NULL,
                last_invoked_at INTEGER,
                invocation_count INTEGER DEFAULT 0,
                total_execution_time_ms INTEGER DEFAULT 0,
                status TEXT DEFAULT 'active',
                version INTEGER DEFAULT 1,
                is_latest BOOLEAN DEFAULT TRUE,
                UNIQUE(name, version)
            )",
            [],
        )?;

        conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_functions_name ON functions(name)",
            [],
        )?;
        conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_functions_status ON functions(status)",
            [],
        )?;
        conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_functions_runtime ON functions(runtime)",
            [],
        )?;

        // API Keys table
        conn.execute(
            "CREATE TABLE IF NOT EXISTS api_keys (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                key TEXT NOT NULL UNIQUE,
                name TEXT NOT NULL,
                permissions TEXT NOT NULL,
                created_at INTEGER NOT NULL,
                expires_at INTEGER,
                status TEXT DEFAULT 'active',
                last_used_at INTEGER
            )",
            [],
        )?;

        conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_api_keys_key ON api_keys(key)",
            [],
        )?;
        conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_api_keys_status ON api_keys(status)",
            [],
        )?;
        conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_functions_version ON functions(name, version)",
            [],
        )?;

        conn.execute(
            "CREATE TABLE IF NOT EXISTS invocations (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                function_id INTEGER NOT NULL,
                request_id TEXT NOT NULL UNIQUE,
                status TEXT NOT NULL,
                started_at INTEGER NOT NULL,
                completed_at INTEGER,
                execution_time_ms INTEGER,
                memory_used_mb INTEGER,
                cold_start INTEGER DEFAULT 0,
                error_message TEXT,
                FOREIGN KEY (function_id) REFERENCES functions(id)
            )",
            [],
        )?;

        conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_invocations_function_id ON invocations(function_id)",
            [],
        )?;
        conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_invocations_started_at ON invocations(started_at)",
            [],
        )?;
        conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_invocations_status ON invocations(status)",
            [],
        )?;

        debug!("Database schema initialized");
        Ok(())
    }

    /// Create a new function
    pub fn create_function(&self, config: FunctionConfig) -> Result<i64> {
        let conn = self.pool.get()?;
        let code_hash = Self::compute_hash(&config.code);
        let now = Self::current_timestamp();
        let environment_json = serde_json::to_string(&config.environment)?;

        // Check if an active function already exists
        let active_exists: bool = conn
            .query_row(
                "SELECT COUNT(*) FROM functions WHERE name = ?1 AND status != 'deleted'",
                params![&config.name],
                |row| row.get(0),
            )
            .map(|count: i64| count > 0)?;

        if active_exists {
            return Err(StorageError::FunctionExists(config.name.clone()));
        }

        // Check if a deleted function exists with this name
        let deleted_id: Option<i64> = conn
            .query_row(
                "SELECT id FROM functions WHERE name = ?1 AND status = 'deleted'",
                params![&config.name],
                |row| row.get(0),
            )
            .optional()?;

        if let Some(id) = deleted_id {
            // Reuse the deleted function row - reset to version 1
            conn.execute(
                "UPDATE functions
                 SET runtime = ?2, handler = ?3, code = ?4, code_hash = ?5,
                     memory_mb = ?6, timeout_ms = ?7, environment = ?8,
                     updated_at = ?9, status = ?10, version = ?11, is_latest = ?12
                 WHERE id = ?1",
                params![
                    id,
                    config.runtime,
                    config.handler,
                    config.code,
                    code_hash,
                    config.memory_mb as i64,
                    config.timeout_ms as i64,
                    environment_json,
                    now,
                    "active",
                    1,    // reset to version 1
                    true  // mark as latest
                ],
            )?;
            info!(
                "Recreated function '{}' with id {} at version 1 (was deleted)",
                config.name, id
            );
            Ok(id)
        } else {
            // Create a new function at version 1
            conn.execute(
                "INSERT INTO functions (
                name, runtime, handler, code, code_hash,
                memory_mb, timeout_ms, environment,
                created_at, updated_at, status, version, is_latest
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13)",
                params![
                    config.name,
                    config.runtime,
                    config.handler,
                    config.code,
                    code_hash,
                    config.memory_mb as i64,
                    config.timeout_ms as i64,
                    environment_json,
                    now,
                    now,
                    "active",
                    1,    // start at version 1
                    true  // mark as latest
                ],
            )?;

            let id = conn.last_insert_rowid();
            info!("Created function '{}' with id {}", config.name, id);
            Ok(id)
        }
    }

    /// Get a function by name
    pub fn get_function(&self, name: &str) -> Result<Option<Function>> {
        let conn = self.pool.get()?;

        let function = conn
            .query_row(
                "SELECT id, name, runtime, handler, code, code_hash,
                        memory_mb, timeout_ms, environment,
                        created_at, updated_at, last_invoked_at,
                        invocation_count, total_execution_time_ms, status,
                        version, is_latest
                 FROM functions
                 WHERE name = ?1 AND status != 'deleted' AND is_latest = TRUE",
                params![name],
                |row| {
                    let environment_json: String = row.get(8)?;
                    let environment: HashMap<String, String> =
                        serde_json::from_str(&environment_json).unwrap_or_default();

                    Ok(Function {
                        id: row.get(0)?,
                        name: row.get(1)?,
                        runtime: row.get(2)?,
                        handler: row.get(3)?,
                        code: row.get(4)?,
                        code_hash: row.get(5)?,
                        memory_mb: row.get::<_, i64>(6)? as u64,
                        timeout_ms: row.get::<_, i64>(7)? as u64,
                        environment,
                        created_at: row.get(9)?,
                        updated_at: row.get(10)?,
                        last_invoked_at: row.get(11)?,
                        invocation_count: row.get(12)?,
                        total_execution_time_ms: row.get(13)?,
                        status: FunctionStatus::from_str(&row.get::<_, String>(14)?).unwrap(),
                        version: row.get(15)?,
                        is_latest: row.get(16)?,
                    })
                },
            )
            .optional()?;

        Ok(function)
    }

    /// Update an existing function
    pub fn update_function(&self, name: &str, config: FunctionConfig) -> Result<()> {
        let conn = self.pool.get()?;
        let code_hash = Self::compute_hash(&config.code);
        let now = Self::current_timestamp();
        let environment_json = serde_json::to_string(&config.environment)?;

        let rows_affected = conn.execute(
            "UPDATE functions
             SET runtime = ?1, handler = ?2, code = ?3, code_hash = ?4,
                 memory_mb = ?5, timeout_ms = ?6, environment = ?7,
                 updated_at = ?8
             WHERE name = ?9 AND status != 'deleted'",
            params![
                config.runtime,
                config.handler,
                config.code,
                code_hash,
                config.memory_mb as i64,
                config.timeout_ms as i64,
                environment_json,
                now,
                name
            ],
        )?;

        if rows_affected == 0 {
            return Err(StorageError::FunctionNotFound(name.to_string()));
        }

        info!("Updated function '{}'", name);
        Ok(())
    }

    /// Delete a function (soft delete)
    pub fn delete_function(&self, name: &str) -> Result<()> {
        let conn = self.pool.get()?;
        let now = Self::current_timestamp();

        let rows_affected = conn.execute(
            "UPDATE functions
             SET status = 'deleted', updated_at = ?1
             WHERE name = ?2 AND status != 'deleted'",
            params![now, name],
        )?;

        if rows_affected == 0 {
            return Err(StorageError::FunctionNotFound(name.to_string()));
        }

        info!("Deleted function '{}'", name);
        Ok(())
    }

    /// Publish a new version of a function (creates a copy with incremented version)
    pub fn publish_version(&self, name: &str, config: FunctionConfig) -> Result<i64> {
        let conn = self.pool.get()?;
        let code_hash = Self::compute_hash(&config.code);
        let now = Self::current_timestamp();
        let environment_json = serde_json::to_string(&config.environment)?;

        // Get the current latest version
        let current_version: i64 = conn
            .query_row(
                "SELECT version FROM functions WHERE name = ?1 AND is_latest = TRUE AND status != 'deleted'",
                params![name],
                |row| row.get(0),
            )?;

        let new_version = current_version + 1;

        // Mark current version as not latest
        conn.execute(
            "UPDATE functions SET is_latest = FALSE WHERE name = ?1 AND is_latest = TRUE",
            params![name],
        )?;

        // Get created_at from original function to preserve it
        let created_at: i64 = conn.query_row(
            "SELECT created_at FROM functions WHERE name = ?1 ORDER BY version ASC LIMIT 1",
            params![name],
            |row| row.get(0),
        )?;

        // Insert new version
        conn.execute(
            "INSERT INTO functions (
                name, runtime, handler, code, code_hash,
                memory_mb, timeout_ms, environment,
                created_at, updated_at, status, version, is_latest,
                invocation_count, total_execution_time_ms
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15)",
            params![
                name,
                config.runtime,
                config.handler,
                config.code,
                code_hash,
                config.memory_mb as i64,
                config.timeout_ms as i64,
                environment_json,
                created_at, // preserve original creation time
                now,
                "active",
                new_version,
                true, // mark as latest
                0,    // reset invocation count for new version
                0     // reset execution time for new version
            ],
        )?;

        let new_id = conn.last_insert_rowid();
        info!(
            "Published function '{}' version {} with id {}",
            name, new_version, new_id
        );
        Ok(new_id)
    }

    /// Get a specific version of a function
    pub fn get_function_by_version(&self, name: &str, version: i64) -> Result<Option<Function>> {
        let conn = self.pool.get()?;

        let function = conn
            .query_row(
                "SELECT id, name, runtime, handler, code, code_hash,
                        memory_mb, timeout_ms, environment,
                        created_at, updated_at, last_invoked_at,
                        invocation_count, total_execution_time_ms, status,
                        version, is_latest
                 FROM functions
                 WHERE name = ?1 AND version = ?2 AND status != 'deleted'",
                params![name, version],
                |row| {
                    let environment_json: String = row.get(8)?;
                    let environment: HashMap<String, String> =
                        serde_json::from_str(&environment_json).unwrap_or_default();

                    Ok(Function {
                        id: row.get(0)?,
                        name: row.get(1)?,
                        runtime: row.get(2)?,
                        handler: row.get(3)?,
                        code: row.get(4)?,
                        code_hash: row.get(5)?,
                        memory_mb: row.get::<_, i64>(6)? as u64,
                        timeout_ms: row.get::<_, i64>(7)? as u64,
                        environment,
                        created_at: row.get(9)?,
                        updated_at: row.get(10)?,
                        last_invoked_at: row.get(11)?,
                        invocation_count: row.get(12)?,
                        total_execution_time_ms: row.get(13)?,
                        status: FunctionStatus::from_str(&row.get::<_, String>(14)?).unwrap(),
                        version: row.get(15)?,
                        is_latest: row.get(16)?,
                    })
                },
            )
            .optional()?;

        Ok(function)
    }

    /// List all versions of a function
    pub fn list_function_versions(&self, name: &str) -> Result<Vec<Function>> {
        let conn = self.pool.get()?;

        let mut stmt = conn.prepare(
            "SELECT id, name, runtime, handler, code, code_hash,
                    memory_mb, timeout_ms, environment,
                    created_at, updated_at, last_invoked_at,
                    invocation_count, total_execution_time_ms, status,
                    version, is_latest
             FROM functions
             WHERE name = ?1 AND status != 'deleted'
             ORDER BY version DESC",
        )?;

        let functions = stmt
            .query_map(params![name], |row| {
                let environment_json: String = row.get(8)?;
                let environment: HashMap<String, String> =
                    serde_json::from_str(&environment_json).unwrap_or_default();

                Ok(Function {
                    id: row.get(0)?,
                    name: row.get(1)?,
                    runtime: row.get(2)?,
                    handler: row.get(3)?,
                    code: row.get(4)?,
                    code_hash: row.get(5)?,
                    memory_mb: row.get::<_, i64>(6)? as u64,
                    timeout_ms: row.get::<_, i64>(7)? as u64,
                    environment,
                    created_at: row.get(9)?,
                    updated_at: row.get(10)?,
                    last_invoked_at: row.get(11)?,
                    invocation_count: row.get(12)?,
                    total_execution_time_ms: row.get(13)?,
                    status: FunctionStatus::from_str(&row.get::<_, String>(14)?).unwrap(),
                    version: row.get(15)?,
                    is_latest: row.get(16)?,
                })
            })?
            .collect::<std::result::Result<Vec<_>, _>>()?;

        Ok(functions)
    }

    /// List all active functions
    pub fn list_functions(&self) -> Result<Vec<Function>> {
        let conn = self.pool.get()?;

        let mut stmt = conn.prepare(
            "SELECT id, name, runtime, handler, code, code_hash,
                    memory_mb, timeout_ms, environment,
                    created_at, updated_at, last_invoked_at,
                    invocation_count, total_execution_time_ms, status,
                    version, is_latest
             FROM functions
             WHERE status = 'active' AND is_latest = TRUE
             ORDER BY name",
        )?;

        let functions = stmt
            .query_map([], |row| {
                let environment_json: String = row.get(8)?;
                let environment: HashMap<String, String> =
                    serde_json::from_str(&environment_json).unwrap_or_default();

                Ok(Function {
                    id: row.get(0)?,
                    name: row.get(1)?,
                    runtime: row.get(2)?,
                    handler: row.get(3)?,
                    code: row.get(4)?,
                    code_hash: row.get(5)?,
                    memory_mb: row.get::<_, i64>(6)? as u64,
                    timeout_ms: row.get::<_, i64>(7)? as u64,
                    environment,
                    created_at: row.get(9)?,
                    updated_at: row.get(10)?,
                    last_invoked_at: row.get(11)?,
                    invocation_count: row.get(12)?,
                    total_execution_time_ms: row.get(13)?,
                    status: FunctionStatus::from_str(&row.get::<_, String>(14)?).unwrap(),
                    version: row.get(15)?,
                    is_latest: row.get(16)?,
                })
            })?
            .collect::<std::result::Result<Vec<_>, _>>()?;

        Ok(functions)
    }

    /// Record a function invocation
    pub fn record_invocation(&self, record: InvocationRecord) -> Result<i64> {
        let conn = self.pool.get()?;

        conn.execute(
            "INSERT INTO invocations (
                function_id, request_id, status, started_at, completed_at,
                execution_time_ms, memory_used_mb, cold_start, error_message
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
            params![
                record.function_id,
                record.request_id,
                record.status.as_str(),
                record.started_at,
                record.completed_at,
                record.execution_time_ms,
                record.memory_used_mb,
                record.cold_start as i32,
                record.error_message,
            ],
        )?;

        let id = conn.last_insert_rowid();
        debug!("Recorded invocation for function_id={}", record.function_id);
        Ok(id)
    }

    /// Update function invocation metrics (called after each invocation)
    pub fn update_invocation_metrics(
        &self,
        function_name: &str,
        execution_time_ms: u64,
    ) -> Result<()> {
        let conn = self.pool.get()?;
        let now = Self::current_timestamp();

        let rows_affected = conn.execute(
            "UPDATE functions
             SET invocation_count = invocation_count + 1,
                 total_execution_time_ms = total_execution_time_ms + ?1,
                 last_invoked_at = ?2
             WHERE name = ?3",
            params![execution_time_ms as i64, now, function_name],
        )?;

        if rows_affected == 0 {
            warn!("Failed to update metrics for function '{}'", function_name);
        }

        Ok(())
    }

    /// Get statistics for a function
    pub fn get_function_stats(&self, name: &str) -> Result<FunctionStats> {
        let conn = self.pool.get()?;

        // Get basic stats from functions table
        let function = self
            .get_function(name)?
            .ok_or_else(|| StorageError::FunctionNotFound(name.to_string()))?;

        // Get detailed stats from invocations table
        let (cold_start_count, error_count, timeout_count) = conn.query_row(
            "SELECT
                SUM(CASE WHEN cold_start = 1 THEN 1 ELSE 0 END) as cold_starts,
                SUM(CASE WHEN status = 'error' THEN 1 ELSE 0 END) as errors,
                SUM(CASE WHEN status = 'timeout' THEN 1 ELSE 0 END) as timeouts
             FROM invocations
             WHERE function_id = ?1",
            params![function.id],
            |row| {
                Ok((
                    row.get(0).unwrap_or(0),
                    row.get(1).unwrap_or(0),
                    row.get(2).unwrap_or(0),
                ))
            },
        )?;

        let avg_execution_time_ms = if function.invocation_count > 0 {
            function.total_execution_time_ms as f64 / function.invocation_count as f64
        } else {
            0.0
        };

        Ok(FunctionStats {
            invocation_count: function.invocation_count,
            total_execution_time_ms: function.total_execution_time_ms,
            avg_execution_time_ms,
            cold_start_count,
            error_count,
            timeout_count,
            last_invoked_at: function.last_invoked_at,
        })
    }

    /// Get invocation history for a function
    pub fn get_invocation_history(
        &self,
        function_name: &str,
        limit: usize,
    ) -> Result<Vec<InvocationRecord>> {
        let function = self
            .get_function(function_name)?
            .ok_or_else(|| StorageError::FunctionNotFound(function_name.to_string()))?;

        let conn = self.pool.get()?;
        let mut stmt = conn.prepare(
            "SELECT function_id, request_id, status, started_at, completed_at,
                    execution_time_ms, memory_used_mb, cold_start, error_message
             FROM invocations
             WHERE function_id = ?1
             ORDER BY started_at DESC
             LIMIT ?2",
        )?;

        let records = stmt
            .query_map(params![function.id, limit], |row| {
                Ok(InvocationRecord {
                    function_id: row.get(0)?,
                    request_id: row.get(1)?,
                    status: InvocationStatus::from_str(&row.get::<_, String>(2)?).unwrap(),
                    started_at: row.get(3)?,
                    completed_at: row.get(4)?,
                    execution_time_ms: row.get(5)?,
                    memory_used_mb: row.get(6)?,
                    cold_start: row.get::<_, i32>(7)? != 0,
                    error_message: row.get(8)?,
                })
            })?
            .collect::<std::result::Result<Vec<_>, _>>()?;

        Ok(records)
    }

    // Helper functions

    fn compute_hash(code: &str) -> String {
        let mut hasher = Sha256::new();
        hasher.update(code.as_bytes());
        format!("{:x}", hasher.finalize())
    }

    fn current_timestamp() -> i64 {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs() as i64
    }

    // API Key operations

    /// Create a new API key
    pub fn create_api_key(&self, request: CreateApiKeyRequest) -> Result<ApiKey> {
        let conn = self.pool.get()?;

        // Generate unique key
        let key = self.generate_api_key();
        let now = Self::current_timestamp();

        // Convert permissions to JSON
        let permissions_json = serde_json::to_string(&request.permissions)?;

        conn.execute(
            "INSERT INTO api_keys (key, name, permissions, created_at, expires_at, status)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![
                &key,
                &request.name,
                &permissions_json,
                now,
                request.expires_at,
                ApiKeyStatus::Active.as_str()
            ],
        )?;

        let id = conn.last_insert_rowid();

        Ok(ApiKey {
            id,
            key,
            name: request.name,
            permissions: request.permissions,
            created_at: now,
            expires_at: request.expires_at,
            status: ApiKeyStatus::Active,
            last_used_at: None,
        })
    }

    /// Get API key by key string
    pub fn get_api_key(&self, key: &str) -> Result<Option<ApiKey>> {
        let conn = self.pool.get()?;

        let result = conn
            .query_row(
                "SELECT id, key, name, permissions, created_at, expires_at, status, last_used_at
             FROM api_keys WHERE key = ?1",
                params![key],
                |row| {
                    let permissions_json: String = row.get(3)?;
                    let permissions: Vec<String> =
                        serde_json::from_str(&permissions_json).unwrap_or_default();

                    Ok(ApiKey {
                        id: row.get(0)?,
                        key: row.get(1)?,
                        name: row.get(2)?,
                        permissions,
                        created_at: row.get(4)?,
                        expires_at: row.get(5)?,
                        status: ApiKeyStatus::from_str(&row.get::<_, String>(6)?).unwrap(),
                        last_used_at: row.get(7)?,
                    })
                },
            )
            .optional()?;

        Ok(result)
    }

    /// List all API keys
    pub fn list_api_keys(&self) -> Result<Vec<ApiKey>> {
        let conn = self.pool.get()?;

        let mut stmt = conn.prepare(
            "SELECT id, key, name, permissions, created_at, expires_at, status, last_used_at
             FROM api_keys ORDER BY created_at DESC",
        )?;

        let keys = stmt
            .query_map([], |row| {
                let permissions_json: String = row.get(3)?;
                let permissions: Vec<String> =
                    serde_json::from_str(&permissions_json).unwrap_or_default();

                Ok(ApiKey {
                    id: row.get(0)?,
                    key: row.get(1)?,
                    name: row.get(2)?,
                    permissions,
                    created_at: row.get(4)?,
                    expires_at: row.get(5)?,
                    status: ApiKeyStatus::from_str(&row.get::<_, String>(6)?).unwrap(),
                    last_used_at: row.get(7)?,
                })
            })?
            .collect::<std::result::Result<Vec<_>, _>>()?;

        Ok(keys)
    }

    /// Revoke API key
    pub fn revoke_api_key(&self, id: i64) -> Result<()> {
        let conn = self.pool.get()?;

        let updated = conn.execute(
            "UPDATE api_keys SET status = ?1 WHERE id = ?2",
            params![ApiKeyStatus::Revoked.as_str(), id],
        )?;

        if updated == 0 {
            return Err(StorageError::NotFound);
        }

        Ok(())
    }

    /// Update last used timestamp
    pub fn update_api_key_last_used(&self, key: &str) -> Result<()> {
        let conn = self.pool.get()?;
        let now = Self::current_timestamp();

        conn.execute(
            "UPDATE api_keys SET last_used_at = ?1 WHERE key = ?2",
            params![now, key],
        )?;

        Ok(())
    }

    /// Generate a secure API key
    fn generate_api_key(&self) -> String {
        let random_bytes: [u8; 32] = rand::random();
        let mut hasher = Sha256::new();
        hasher.update(random_bytes);
        let result = hasher.finalize();
        format!("nl_{}", hex::encode(result))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_config() -> FunctionConfig {
        FunctionConfig {
            name: "test-function".to_string(),
            runtime: "python3.11".to_string(),
            handler: "index.handler".to_string(),
            code: "def handler(event):\n    return {'statusCode': 200}".to_string(),
            memory_mb: 128,
            timeout_ms: 30000,
            environment: HashMap::new(),
        }
    }

    #[test]
    fn test_create_and_get_function() {
        let storage = StorageManager::new_in_memory().unwrap();
        let config = test_config();

        // Create function
        let id = storage.create_function(config.clone()).unwrap();
        assert!(id > 0);

        // Get function
        let function = storage.get_function(&config.name).unwrap().unwrap();
        assert_eq!(function.name, config.name);
        assert_eq!(function.runtime, config.runtime);
        assert_eq!(function.code, config.code);
        assert_eq!(function.status, FunctionStatus::Active);
    }

    #[test]
    fn test_create_duplicate_function() {
        let storage = StorageManager::new_in_memory().unwrap();
        let config = test_config();

        storage.create_function(config.clone()).unwrap();
        let result = storage.create_function(config);
        assert!(matches!(result, Err(StorageError::FunctionExists(_))));
    }

    #[test]
    fn test_update_function() {
        let storage = StorageManager::new_in_memory().unwrap();
        let mut config = test_config();

        storage.create_function(config.clone()).unwrap();

        // Update code
        config.code = "def handler(event):\n    return {'statusCode': 201}".to_string();
        storage
            .update_function(&config.name, config.clone())
            .unwrap();

        // Verify update
        let function = storage.get_function(&config.name).unwrap().unwrap();
        assert_eq!(function.code, config.code);
    }

    #[test]
    fn test_delete_function() {
        let storage = StorageManager::new_in_memory().unwrap();
        let config = test_config();

        storage.create_function(config.clone()).unwrap();
        storage.delete_function(&config.name).unwrap();

        // Function should not be retrievable after deletion
        let function = storage.get_function(&config.name).unwrap();
        assert!(function.is_none());
    }

    #[test]
    fn test_list_functions() {
        let storage = StorageManager::new_in_memory().unwrap();

        // Create multiple functions
        for i in 1..=3 {
            let mut config = test_config();
            config.name = format!("test-function-{}", i);
            storage.create_function(config).unwrap();
        }

        let functions = storage.list_functions().unwrap();
        assert_eq!(functions.len(), 3);
    }

    #[test]
    fn test_invocation_metrics() {
        let storage = StorageManager::new_in_memory().unwrap();
        let config = test_config();

        storage.create_function(config.clone()).unwrap();

        // Update metrics
        storage
            .update_invocation_metrics(&config.name, 100)
            .unwrap();
        storage
            .update_invocation_metrics(&config.name, 200)
            .unwrap();

        let function = storage.get_function(&config.name).unwrap().unwrap();
        assert_eq!(function.invocation_count, 2);
        assert_eq!(function.total_execution_time_ms, 300);
    }

    #[test]
    fn test_function_stats() {
        let storage = StorageManager::new_in_memory().unwrap();
        let config = test_config();

        let id = storage.create_function(config.clone()).unwrap();

        // Record some invocations
        for i in 0..5 {
            storage
                .record_invocation(InvocationRecord {
                    function_id: id,
                    request_id: format!("req-{}", i),
                    status: if i < 4 {
                        InvocationStatus::Success
                    } else {
                        InvocationStatus::Error
                    },
                    started_at: StorageManager::current_timestamp(),
                    completed_at: Some(StorageManager::current_timestamp()),
                    execution_time_ms: Some(100),
                    memory_used_mb: Some(64),
                    cold_start: i == 0,
                    error_message: None,
                })
                .unwrap();
        }

        storage
            .update_invocation_metrics(&config.name, 500)
            .unwrap();

        let stats = storage.get_function_stats(&config.name).unwrap();
        assert_eq!(stats.invocation_count, 1);
        assert_eq!(stats.cold_start_count, 1);
        assert_eq!(stats.error_count, 1);
    }
}
