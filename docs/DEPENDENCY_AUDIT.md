# Dependency Audit and Update Plan

**Date**: October 18, 2025  
**Rust Version**: 1.90.0 (supports edition 2024)

## Current Status

### Edition Status
- ✅ Main workspace: **2024**
- ✅ VMM crate: **2024**
- ✅ Scheduler crate: **2024**
- ❌ API Server: **2021** → needs update
- ❌ Runtime: **2021** → needs update
- ❌ Storage: **2021** → needs update
- ❌ Benchmarks: **2021** → needs update

## Dependency Analysis

### 1. Web Framework Choice

**Current: Mixed (Axum + Actix-web)**
- Main: actix-web 4.8
- API Server: axum 0.7
- API Server also has: hyper 0.14 (outdated)

**Recommendation: Standardize on Axum 0.8**
- ✅ Modern async architecture
- ✅ Better with Tokio integration
- ✅ Type-safe routing
- ✅ More actively maintained than Actix
- ✅ Latest version: 0.8.x (November 2024)
- ⚠️ Remove actix-web from main workspace

### 2. Async Runtime

**Current: tokio 1.40-1.48 (mixed versions)**
- Main: 1.48
- Others: 1.40-1.41

**Recommendation: Standardize on tokio 1.45+**
- Latest stable: 1.45.x
- Consistent version across all crates

### 3. HTTP Client

**Current: Mixed**
- Benchmarks: reqwest 0.11
- API Server: hyper 0.14 (outdated, superseded by 1.x)

**Recommendation: Upgrade to reqwest 0.12 + hyper 1.x**
- reqwest 0.12 uses hyper 1.x internally
- Better HTTP/2 support
- Improved performance

### 4. Database (Storage)

**Current: rusqlite 0.31**
- ✅ Latest version
- ✅ Good choice for embedded DB
- Alternative: sqlx 0.8 (async, but adds complexity)

**Recommendation: Keep rusqlite, but update connection pooling**
- r2d2 0.8 → consider deadpool-sqlite 0.8 for async
- OR keep r2d2 for simplicity (sync is fine for local DB)

### 5. AWS SDK

**Current: aws-sdk-lambda 1.9, aws-config 1.0**

**Recommendation: Update to latest**
- aws-sdk-lambda: 1.100+ (latest)
- aws-config: 1.8+ (latest with behavior-version-latest)
- Already using latest in benchmarks ✅

### 6. Serialization

**Current: serde 1.0, serde_json 1.0**
- ✅ Latest versions
- ✅ Industry standard

### 7. Error Handling

**Current: Mixed thiserror versions**
- Main/Scheduler/VMM: thiserror 2.0.17
- Others: thiserror 1.0

**Recommendation: Standardize on thiserror 2.0**
- Latest major version with better error handling

### 8. Logging

**Current: tracing 0.1**
- ✅ Latest version
- ⚠️ Missing tracing-subscriber in most crates

**Recommendation: Add tracing ecosystem**
- tracing-subscriber 0.3
- tracing-appender 0.2
- Better structured logging

### 9. CLI Tools

**Current: clap 4.4**
- ✅ Modern version
- Latest: 4.5.x (minor updates)

### 10. Crypto/Hashing

**Current: Mixed**
- Storage: sha2 0.10
- Runtime: md5 0.7

**Recommendation: Standardize on sha2 0.10**
- ✅ sha2 is better choice than md5
- md5 is only for cache keys (consider upgrading to blake3 0.3 for speed)

## Dependency Recommendations by Functionality

### Process Management (Runtime)
- Current: Direct Command/Stdio
- ✅ Good choice, no changes needed

### Time Handling
- Current: chrono 0.4 (storage only)
- Alternative: time 0.3 (faster, but chrono has more features)
- ✅ Keep chrono for storage

### UUID Generation
- Current: uuid 1.6
- ✅ Latest version, good choice

### Testing
- Current: tower 0.4, futures 0.3
- ✅ Good versions

## Update Plan

### Phase 1: Edition Updates (All crates to 2024)
1. ✅ crates/api-server/Cargo.toml
2. ✅ crates/runtime/Cargo.toml
3. ✅ crates/storage/Cargo.toml
4. ✅ benchmarks/Cargo.toml

### Phase 2: Core Dependency Updates
1. Standardize tokio → 1.45+
2. Upgrade thiserror → 2.0+ everywhere
3. Remove actix-web from main, keep only Axum
4. Upgrade hyper → 1.x (via axum 0.8)

### Phase 3: Optional Improvements
1. Add tracing-subscriber for better logging
2. Consider blake3 instead of md5 for runtime hashing
3. Add tokio-console support for debugging

## Breaking Changes to Consider

### Axum 0.7 → 0.8
- Handler trait changes
- New State extraction
- Better type inference

### Hyper 0.14 → 1.x
- Different API surface
- Used internally by axum/reqwest

### Thiserror 1.0 → 2.0
- Minimal breaking changes
- Better error formatting

## Best Practice Recommendations

### 1. Workspace Dependencies
Use `[workspace.dependencies]` to centralize versions:

```toml
[workspace.dependencies]
tokio = { version = "1.45", features = ["full"] }
serde = { version = "1.0", features = ["derive"] }
anyhow = "1.0"
thiserror = "2.0"
```

### 2. Feature Flags
Be explicit about features to minimize bloat:
```toml
tokio = { version = "1.45", features = ["rt-multi-thread", "macros", "sync", "time"] }
```

### 3. Lock File
Commit Cargo.lock for binary projects (servers, CLIs)

## Action Items

1. **Update all editions to 2024** ✅
2. **Standardize tokio version** → 1.45
3. **Upgrade thiserror** → 2.0 everywhere
4. **Remove actix-web** → use only Axum
5. **Add workspace dependencies** → centralize versions
6. **Test all changes** → run full test suite
7. **Update documentation** → reflect new dependencies

## Validation Commands

```bash
# Check outdated dependencies
cargo outdated

# Security audit
cargo audit

# Unused dependencies
cargo machete

# Build all crates
cargo build --workspace

# Test all crates
cargo test --workspace

# Check for issues
cargo clippy --workspace -- -D warnings
```
