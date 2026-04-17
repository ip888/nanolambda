# NanoLambda Edge Implementation Plan

## Phase 1: Monetization Foundation (Week 1-2)

### 1.1 User Authentication System

**Files to Create:**
```
crates/
├── auth/
│   ├── Cargo.toml
│   ├── src/
│   │   ├── lib.rs           # Auth module exports
│   │   ├── user.rs          # User model & CRUD
│   │   ├── api_key.rs       # API key generation & validation
│   │   ├── jwt.rs           # JWT token handling
│   │   ├── session.rs       # Session management
│   │   └── middleware.rs    # Auth middleware for Axum
```

**User Model:**
```rust
// crates/auth/src/user.rs
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct User {
    pub id: String,
    pub email: String,
    pub password_hash: String,
    pub plan: Plan,
    pub created_at: DateTime<Utc>,
    pub usage: Usage,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Plan {
    Free,
    Pro,
    Scale,
    Enterprise,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Usage {
    pub invocations: u64,
    pub vectors_stored: u64,
    pub bandwidth_bytes: u64,
    pub last_reset: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiKey {
    pub id: String,
    pub user_id: String,
    pub key_hash: String,
    pub name: String,
    pub permissions: Vec<Permission>,
    pub created_at: DateTime<Utc>,
    pub last_used: Option<DateTime<Utc>>,
}
```

**API Endpoints:**
```
POST /auth/register         # Create account
POST /auth/login            # Get JWT token
POST /auth/logout           # Invalidate session
GET  /auth/me               # Get current user
POST /auth/api-keys         # Create API key
GET  /auth/api-keys         # List API keys
DELETE /auth/api-keys/:id   # Revoke API key
```

### 1.2 Usage Metering & Limits

**Files to Create:**
```
crates/
├── metering/
│   ├── Cargo.toml
│   ├── src/
│   │   ├── lib.rs           # Metering exports
│   │   ├── tracker.rs       # Usage tracking
│   │   ├── limits.rs        # Plan limits
│   │   └── aggregator.rs    # Usage aggregation
```

**Plan Limits:**
```rust
// crates/metering/src/limits.rs
pub struct PlanLimits {
    pub invocations_per_month: u64,
    pub max_memory_mb: u32,
    pub max_timeout_seconds: u32,
    pub max_functions: u32,
    pub bandwidth_gb: u64,
}

pub const FREE_LIMITS: PlanLimits = PlanLimits {
    invocations_per_month: 100_000,
    max_memory_mb: 512,
    max_timeout_seconds: 10,
    max_functions: 5,
    bandwidth_gb: 100,
};

pub const PRO_LIMITS: PlanLimits = PlanLimits {
    invocations_per_month: 1_000_000,
    max_memory_mb: 1024,
    max_timeout_seconds: 30,
    max_functions: 50,
    bandwidth_gb: 500,
};
```

### 1.3 Stripe Integration

**Files to Create:**
```
crates/
├── billing/
│   ├── Cargo.toml
│   ├── src/
│   │   ├── lib.rs           # Billing exports
│   │   ├── stripe.rs        # Stripe API client
│   │   ├── subscription.rs  # Subscription management
│   │   ├── invoice.rs       # Invoice handling
│   │   └── webhook.rs       # Stripe webhook handler
```

**Stripe Products:**
```rust
// crates/billing/src/stripe.rs
pub const STRIPE_PRODUCTS: &[(&str, &str)] = &[
    ("price_pro_monthly", "Pro Plan - $29/month"),
    ("price_scale_monthly", "Scale Plan - $149/month"),
];

pub async fn create_checkout_session(
    user_id: &str,
    price_id: &str,
) -> Result<String, BillingError> {
    let client = stripe::Client::new(env::var("STRIPE_SECRET_KEY")?);
    
    let session = stripe::CheckoutSession::create(&client, CreateCheckoutSession {
        success_url: Some("https://nanolambda.com/dashboard?success=true"),
        cancel_url: Some("https://nanolambda.com/pricing"),
        mode: Some(stripe::CheckoutSessionMode::Subscription),
        line_items: Some(vec![CreateCheckoutSessionLineItems {
            price: Some(price_id.to_string()),
            quantity: Some(1),
        }]),
        ..Default::default()
    }).await?;
    
    Ok(session.url.unwrap())
}
```

---

## Phase 2: Competitive Features (Week 3-4)

### 2.1 Scheduled Functions (Cron)

**Files to Create:**
```
crates/
├── scheduler/src/
│   ├── cron.rs              # Cron expression parser
│   ├── job.rs               # Scheduled job model
│   ├── executor.rs          # Job execution
│   └── queue.rs             # Job queue management
```

**Cron Model:**
```rust
// crates/scheduler/src/job.rs
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScheduledJob {
    pub id: String,
    pub function_id: String,
    pub user_id: String,
    pub cron_expression: String,  // "0 */5 * * *" = every 5 min
    pub timezone: String,
    pub payload: Option<serde_json::Value>,
    pub enabled: bool,
    pub last_run: Option<DateTime<Utc>>,
    pub next_run: DateTime<Utc>,
    pub created_at: DateTime<Utc>,
}

// API Endpoints
POST /v1/schedules              # Create scheduled job
GET  /v1/schedules              # List scheduled jobs
GET  /v1/schedules/:id          # Get scheduled job
PUT  /v1/schedules/:id          # Update scheduled job
DELETE /v1/schedules/:id        # Delete scheduled job
GET  /v1/schedules/:id/runs     # Get execution history
```

**Cron Executor (Background Task):**
```rust
// crates/scheduler/src/executor.rs
pub async fn run_scheduler(storage: Arc<StorageManager>) {
    let mut interval = tokio::time::interval(Duration::from_secs(1));
    
    loop {
        interval.tick().await;
        
        // Get jobs due to run
        let due_jobs = storage.get_due_scheduled_jobs(Utc::now()).await?;
        
        for job in due_jobs {
            // Spawn execution task
            tokio::spawn(async move {
                let result = execute_function(&job.function_id, job.payload).await;
                
                // Record execution
                storage.record_scheduled_run(&job.id, &result).await?;
                
                // Update next run time
                let next_run = calculate_next_run(&job.cron_expression, &job.timezone);
                storage.update_next_run(&job.id, next_run).await?;
            });
        }
    }
}
```

### 2.2 WebSocket Support

**Files to Create:**
```
crates/
├── websocket/
│   ├── Cargo.toml
│   ├── src/
│   │   ├── lib.rs           # WebSocket exports
│   │   ├── handler.rs       # Connection handler
│   │   ├── room.rs          # Room/channel management
│   │   └── message.rs       # Message types
```

**WebSocket Handler:**
```rust
// crates/websocket/src/handler.rs
pub async fn websocket_handler(
    ws: WebSocketUpgrade,
    State(state): State<AppState>,
    Query(params): Query<WsParams>,
) -> impl IntoResponse {
    ws.on_upgrade(move |socket| handle_socket(socket, state, params))
}

async fn handle_socket(socket: WebSocket, state: AppState, params: WsParams) {
    let (sender, receiver) = socket.split();
    
    // Authenticate
    let user = authenticate_ws(&params.token, &state).await?;
    
    // Join room
    state.rooms.join(&params.room, user.id.clone(), sender).await;
    
    // Handle incoming messages
    while let Some(msg) = receiver.next().await {
        match msg {
            Ok(Message::Text(text)) => {
                // Trigger function execution
                let result = invoke_function(
                    &params.function_id,
                    serde_json::from_str(&text)?,
                ).await;
                
                // Broadcast result
                state.rooms.broadcast(&params.room, &result).await;
            }
            _ => {}
        }
    }
    
    // Leave room on disconnect
    state.rooms.leave(&params.room, &user.id).await;
}
```

### 2.3 Auto-Scaling (Process Pool Enhancement)

**Enhanced Pool Configuration:**
```rust
// crates/runtime/src/pool.rs
#[derive(Debug, Clone)]
pub struct AutoScaleConfig {
    pub min_instances: usize,
    pub max_instances: usize,
    pub scale_up_threshold: f64,   // CPU utilization %
    pub scale_down_threshold: f64,
    pub scale_up_cooldown: Duration,
    pub scale_down_cooldown: Duration,
}

impl ProcessPool {
    pub async fn auto_scale(&self, metrics: &PoolMetrics) {
        let utilization = metrics.active_processes as f64 / self.config.max_instances as f64;
        
        if utilization > self.config.scale_up_threshold {
            self.scale_up().await;
        } else if utilization < self.config.scale_down_threshold {
            self.scale_down().await;
        }
    }
    
    async fn scale_up(&self) {
        let current = self.processes.read().await.len();
        if current < self.config.max_instances {
            let new_count = (current + (current / 2)).min(self.config.max_instances);
            self.spawn_processes(new_count - current).await;
        }
    }
}
```

### 2.4 QuartzDB Integration

**SDK for Function Context:**
```rust
// crates/runtime/src/context.rs
pub struct FunctionContext {
    pub request_id: String,
    pub function_name: String,
    pub memory_limit_mb: u32,
    pub timeout_ms: u64,
    pub remaining_time_ms: u64,
    
    // QuartzDB integration
    pub quartzdb: QuartzDBClient,
}

impl FunctionContext {
    pub fn to_python_dict(&self) -> String {
        format!(r#"
class context:
    request_id = "{}"
    function_name = "{}"
    memory_limit_mb = {}
    
    class quartzdb:
        @staticmethod
        def insert(id, vector, metadata=None):
            return _quartzdb_insert(id, vector, metadata)
        
        @staticmethod
        def search(vector, k=10):
            return _quartzdb_search(vector, k)
        
        @staticmethod
        def delete(id):
            return _quartzdb_delete(id)
"#, self.request_id, self.function_name, self.memory_limit_mb)
    }
}
```

---

## Database Schema Updates

```sql
-- Users table
CREATE TABLE users (
    id TEXT PRIMARY KEY,
    email TEXT UNIQUE NOT NULL,
    password_hash TEXT NOT NULL,
    plan TEXT NOT NULL DEFAULT 'free',
    stripe_customer_id TEXT,
    created_at DATETIME DEFAULT CURRENT_TIMESTAMP
);

-- API Keys table
CREATE TABLE api_keys (
    id TEXT PRIMARY KEY,
    user_id TEXT NOT NULL REFERENCES users(id),
    key_hash TEXT NOT NULL,
    name TEXT NOT NULL,
    permissions TEXT NOT NULL, -- JSON array
    created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
    last_used DATETIME
);

-- Usage tracking table
CREATE TABLE usage_records (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    user_id TEXT NOT NULL REFERENCES users(id),
    type TEXT NOT NULL, -- 'invocation', 'vector_query', 'bandwidth'
    amount INTEGER NOT NULL,
    recorded_at DATETIME DEFAULT CURRENT_TIMESTAMP
);

-- Scheduled jobs table
CREATE TABLE scheduled_jobs (
    id TEXT PRIMARY KEY,
    function_id TEXT NOT NULL REFERENCES functions(id),
    user_id TEXT NOT NULL REFERENCES users(id),
    cron_expression TEXT NOT NULL,
    timezone TEXT NOT NULL DEFAULT 'UTC',
    payload TEXT, -- JSON
    enabled BOOLEAN NOT NULL DEFAULT true,
    last_run DATETIME,
    next_run DATETIME NOT NULL,
    created_at DATETIME DEFAULT CURRENT_TIMESTAMP
);

-- Scheduled job runs table
CREATE TABLE scheduled_runs (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    job_id TEXT NOT NULL REFERENCES scheduled_jobs(id),
    status TEXT NOT NULL, -- 'success', 'failure'
    duration_ms INTEGER,
    error TEXT,
    started_at DATETIME NOT NULL,
    completed_at DATETIME
);

-- Subscriptions table
CREATE TABLE subscriptions (
    id TEXT PRIMARY KEY,
    user_id TEXT NOT NULL REFERENCES users(id),
    stripe_subscription_id TEXT NOT NULL,
    plan TEXT NOT NULL,
    status TEXT NOT NULL, -- 'active', 'cancelled', 'past_due'
    current_period_start DATETIME,
    current_period_end DATETIME,
    created_at DATETIME DEFAULT CURRENT_TIMESTAMP
);
```

---

## API Routes Summary

```rust
// crates/api-server/src/routes.rs
pub fn create_router(state: AppState) -> Router {
    Router::new()
        // Health
        .route("/health", get(health_check))
        
        // Auth (Phase 1)
        .route("/auth/register", post(register))
        .route("/auth/login", post(login))
        .route("/auth/logout", post(logout))
        .route("/auth/me", get(get_current_user))
        .route("/auth/api-keys", get(list_api_keys).post(create_api_key))
        .route("/auth/api-keys/:id", delete(delete_api_key))
        
        // Functions (existing)
        .route("/v1/functions", get(list_functions).post(create_function))
        .route("/v1/functions/:id", get(get_function).put(update_function).delete(delete_function))
        .route("/v1/functions/:id/invoke", post(invoke_function))
        
        // Schedules (Phase 2)
        .route("/v1/schedules", get(list_schedules).post(create_schedule))
        .route("/v1/schedules/:id", get(get_schedule).put(update_schedule).delete(delete_schedule))
        .route("/v1/schedules/:id/runs", get(list_schedule_runs))
        
        // WebSocket (Phase 2)
        .route("/v1/ws/:function_id", get(websocket_handler))
        
        // Billing (Phase 1)
        .route("/billing/checkout", post(create_checkout))
        .route("/billing/portal", get(customer_portal))
        .route("/billing/webhook", post(stripe_webhook))
        
        // Usage (Phase 1)
        .route("/usage", get(get_usage))
        .route("/usage/history", get(get_usage_history))
        
        .with_state(state)
}
```

---

## Cloudflare Workers Deployment

### Project Structure for Edge Deployment

```
nanolambda-edge/
├── wrangler.toml            # Cloudflare Workers config
├── Cargo.toml               # Rust workspace
├── src/
│   ├── lib.rs               # Worker entry point
│   ├── router.rs            # Request routing
│   ├── executor.rs          # Function execution (V8)
│   └── integrations/
│       ├── quartzdb.rs      # QuartzDB client
│       └── workers_ai.rs    # Workers AI client
└── bindings.d.ts            # TypeScript bindings
```

### wrangler.toml Configuration

```toml
name = "nanolambda-edge"
main = "build/worker/shim.mjs"
compatibility_date = "2024-01-01"

[build]
command = "cargo install -q worker-build && worker-build --release"

[[kv_namespaces]]
binding = "FUNCTIONS"
id = "xxx"

[[durable_objects.bindings]]
name = "EXECUTOR"
class_name = "FunctionExecutor"

[[durable_objects.bindings]]
name = "QUARTZDB"
class_name = "VectorIndex"

[vars]
ENVIRONMENT = "production"

[[r2_buckets]]
binding = "PACKAGES"
bucket_name = "nanolambda-packages"

[ai]
binding = "AI"
```

---

## Implementation Order

### Week 1
1. ✅ User model & database schema
2. ✅ Registration & login endpoints
3. ✅ API key generation & validation
4. ✅ Auth middleware integration

### Week 2
5. ✅ Usage metering & tracking
6. ✅ Plan limits enforcement
7. ✅ Stripe checkout integration
8. ✅ Subscription webhook handling

### Week 3
9. ✅ Cron expression parser
10. ✅ Scheduled jobs CRUD
11. ✅ Background scheduler task
12. ✅ Execution history tracking

### Week 4
13. ✅ WebSocket handler
14. ✅ Room/channel management
15. ✅ Auto-scaling logic
16. ✅ QuartzDB context integration

### Week 5-6
17. ✅ Cloudflare Workers deployment
18. ✅ Unified dashboard
19. ✅ Documentation updates
20. ✅ Beta launch preparation
