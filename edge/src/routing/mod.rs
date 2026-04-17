//! Smart Edge Routing System
//!
//! Intelligent request routing for optimal latency and load distribution:
//! - Geo-aware routing to nearest edge location
//! - Load-based routing with spillover
//! - A/B testing and canary deployments
//! - Request coalescing for duplicate requests

pub mod geo;

use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};
use std::time::{Duration, Instant};
use worker::console_log;

/// Geographic region identifiers
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Region {
    /// North America East
    NaEast,
    /// North America West
    NaWest,
    /// Europe West
    EuWest,
    /// Europe Central
    EuCentral,
    /// Asia Pacific (Singapore)
    ApSoutheast,
    /// Asia Pacific (Tokyo)
    ApNortheast,
    /// South America
    SaEast,
    /// Australia
    OcAustralia,
}

impl Region {
    /// Get region from CF-IPCountry header
    pub fn from_country_code(code: &str) -> Self {
        match code {
            "US" | "CA" => Self::NaEast, // Would be more granular in production
            "MX" | "BR" | "AR" => Self::SaEast,
            "GB" | "FR" | "ES" | "PT" | "IE" => Self::EuWest,
            "DE" | "PL" | "AT" | "CH" | "NL" | "BE" => Self::EuCentral,
            "JP" | "KR" => Self::ApNortheast,
            "SG" | "MY" | "TH" | "ID" | "PH" => Self::ApSoutheast,
            "AU" | "NZ" => Self::OcAustralia,
            _ => Self::NaEast, // Default
        }
    }

    /// Get approximate latency to another region (ms)
    pub fn latency_to(&self, other: &Region) -> u32 {
        if self == other {
            return 1; // Local
        }

        // Approximate cross-region latencies
        match (self, other) {
            (Region::NaEast, Region::NaWest) | (Region::NaWest, Region::NaEast) => 70,
            (Region::NaEast, Region::EuWest) | (Region::EuWest, Region::NaEast) => 80,
            (Region::NaWest, Region::ApNortheast) | (Region::ApNortheast, Region::NaWest) => 120,
            (Region::EuWest, Region::EuCentral) | (Region::EuCentral, Region::EuWest) => 20,
            _ => 150, // Default cross-region
        }
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            Self::NaEast => "na-east",
            Self::NaWest => "na-west",
            Self::EuWest => "eu-west",
            Self::EuCentral => "eu-central",
            Self::ApSoutheast => "ap-southeast",
            Self::ApNortheast => "ap-northeast",
            Self::SaEast => "sa-east",
            Self::OcAustralia => "oc-australia",
        }
    }
}

/// Edge location status
#[derive(Debug)]
pub struct EdgeLocation {
    /// Region identifier
    pub region: Region,
    /// Current load (0-100)
    pub load: AtomicUsize,
    /// Active connections
    pub connections: AtomicU64,
    /// Health status
    pub healthy: bool,
    /// Available capacity
    pub capacity: u32,
    /// Custom metadata
    pub metadata: HashMap<String, String>,
}

impl EdgeLocation {
    pub fn new(region: Region, capacity: u32) -> Self {
        Self {
            region,
            load: AtomicUsize::new(0),
            connections: AtomicU64::new(0),
            healthy: true,
            capacity,
            metadata: HashMap::new(),
        }
    }

    /// Check if location can accept more requests
    pub fn can_accept(&self) -> bool {
        self.healthy && self.load.load(Ordering::Relaxed) < 90
    }

    /// Record a request
    pub fn record_request(&self) {
        self.connections.fetch_add(1, Ordering::Relaxed);
    }

    /// Complete a request
    pub fn complete_request(&self) {
        self.connections.fetch_sub(1, Ordering::Relaxed);
    }
}

/// Routing decision
#[derive(Debug, Clone)]
pub struct RoutingDecision {
    /// Selected target region
    pub target: Region,
    /// Routing strategy used
    pub strategy: RoutingStrategy,
    /// Estimated latency (ms)
    pub estimated_latency: u32,
    /// Whether this is a fallback route
    pub is_fallback: bool,
    /// Routing metadata
    pub metadata: HashMap<String, String>,
}

/// Available routing strategies
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RoutingStrategy {
    /// Route to nearest healthy edge
    Nearest,
    /// Round-robin load balancing
    RoundRobin,
    /// Least connections
    LeastConnections,
    /// Weighted random
    WeightedRandom,
    /// A/B test routing
    AbTest,
    /// Canary deployment
    Canary,
    /// Geographic affinity
    GeoAffinity,
}

/// Routing configuration
#[derive(Debug, Clone)]
pub struct RoutingConfig {
    /// Default strategy
    pub default_strategy: RoutingStrategy,
    /// Enable request coalescing
    pub enable_coalescing: bool,
    /// Coalescing window (ms)
    pub coalescing_window_ms: u32,
    /// Maximum retries
    pub max_retries: u32,
    /// Retry backoff (ms)
    pub retry_backoff_ms: u32,
    /// Health check interval (seconds)
    pub health_check_interval_secs: u32,
}

impl Default for RoutingConfig {
    fn default() -> Self {
        Self {
            default_strategy: RoutingStrategy::Nearest,
            enable_coalescing: true,
            coalescing_window_ms: 50,
            max_retries: 3,
            retry_backoff_ms: 100,
            health_check_interval_secs: 30,
        }
    }
}

/// Smart edge router
pub struct EdgeRouter {
    /// Configuration
    config: RoutingConfig,
    /// Known edge locations
    locations: HashMap<Region, EdgeLocation>,
    /// Round-robin counter
    rr_counter: AtomicUsize,
    /// Routing statistics
    stats: RouterStats,
    /// A/B test configurations
    ab_tests: HashMap<String, AbTestConfig>,
    /// Canary configurations
    canary_configs: HashMap<String, CanaryConfig>,
}

/// Router statistics
#[derive(Debug, Default)]
pub struct RouterStats {
    /// Total routing decisions
    pub total_decisions: AtomicU64,
    /// Decisions by strategy
    pub decisions_by_strategy: [AtomicU64; 7],
    /// Fallback routes used
    pub fallbacks: AtomicU64,
    /// Failed routes
    pub failures: AtomicU64,
}

/// A/B test configuration
#[derive(Debug, Clone)]
pub struct AbTestConfig {
    /// Test name
    pub name: String,
    /// Variant A percentage (0-100)
    pub variant_a_percent: u8,
    /// Variant A target
    pub variant_a_target: Region,
    /// Variant B target
    pub variant_b_target: Region,
    /// Test active
    pub active: bool,
}

/// Canary deployment configuration
#[derive(Debug, Clone)]
pub struct CanaryConfig {
    /// Deployment name
    pub name: String,
    /// Canary percentage (0-100)
    pub canary_percent: u8,
    /// Canary target
    pub canary_target: Region,
    /// Production target
    pub production_target: Region,
    /// Auto-rollback on error threshold
    pub error_threshold: f32,
}

impl EdgeRouter {
    /// Create a new router
    pub fn new(config: RoutingConfig) -> Self {
        Self {
            config,
            locations: HashMap::new(),
            rr_counter: AtomicUsize::new(0),
            stats: RouterStats::default(),
            ab_tests: HashMap::new(),
            canary_configs: HashMap::new(),
        }
    }

    /// Register an edge location
    pub fn register_location(&mut self, location: EdgeLocation) {
        self.locations.insert(location.region, location);
    }

    /// Make a routing decision
    pub fn route(&self, request: &RoutingRequest) -> RoutingDecision {
        self.stats.total_decisions.fetch_add(1, Ordering::Relaxed);

        // Check for A/B test
        if let Some(test_name) = &request.ab_test {
            if let Some(test) = self.ab_tests.get(test_name) {
                if test.active {
                    return self.route_ab_test(request, test);
                }
            }
        }

        // Check for canary
        if let Some(canary_name) = &request.canary {
            if let Some(canary) = self.canary_configs.get(canary_name) {
                return self.route_canary(request, canary);
            }
        }

        // Use configured strategy
        match request.strategy.unwrap_or(self.config.default_strategy) {
            RoutingStrategy::Nearest => self.route_nearest(request),
            RoutingStrategy::RoundRobin => self.route_round_robin(request),
            RoutingStrategy::LeastConnections => self.route_least_connections(request),
            RoutingStrategy::WeightedRandom => self.route_weighted_random(request),
            RoutingStrategy::GeoAffinity => self.route_geo_affinity(request),
            _ => self.route_nearest(request),
        }
    }

    /// Route to nearest healthy location
    fn route_nearest(&self, request: &RoutingRequest) -> RoutingDecision {
        let client_region = request.client_region;
        let mut best_latency = u32::MAX;
        let mut best_region = client_region;
        let mut is_fallback = false;

        // Find nearest healthy location
        for (region, location) in &self.locations {
            if !location.can_accept() {
                continue;
            }

            let latency = client_region.latency_to(region);
            if latency < best_latency {
                best_latency = latency;
                best_region = *region;
            }
        }

        // Fallback if no healthy location found
        if best_latency == u32::MAX {
            best_region = client_region;
            best_latency = 1;
            is_fallback = true;
            self.stats.fallbacks.fetch_add(1, Ordering::Relaxed);
        }

        self.record_strategy(RoutingStrategy::Nearest);

        RoutingDecision {
            target: best_region,
            strategy: RoutingStrategy::Nearest,
            estimated_latency: best_latency,
            is_fallback,
            metadata: HashMap::new(),
        }
    }

    /// Round-robin routing
    fn route_round_robin(&self, _request: &RoutingRequest) -> RoutingDecision {
        let healthy: Vec<_> = self
            .locations
            .iter()
            .filter(|(_, loc)| loc.can_accept())
            .collect();

        if healthy.is_empty() {
            return self.fallback_decision();
        }

        let idx = self.rr_counter.fetch_add(1, Ordering::Relaxed) % healthy.len();
        let (region, _) = healthy[idx];

        self.record_strategy(RoutingStrategy::RoundRobin);

        RoutingDecision {
            target: *region,
            strategy: RoutingStrategy::RoundRobin,
            estimated_latency: 50, // Average estimate
            is_fallback: false,
            metadata: HashMap::new(),
        }
    }

    /// Route to location with least connections
    fn route_least_connections(&self, _request: &RoutingRequest) -> RoutingDecision {
        let mut min_connections = u64::MAX;
        let mut best_region = Region::NaEast;

        for (region, location) in &self.locations {
            if !location.can_accept() {
                continue;
            }

            let connections = location.connections.load(Ordering::Relaxed);
            if connections < min_connections {
                min_connections = connections;
                best_region = *region;
            }
        }

        self.record_strategy(RoutingStrategy::LeastConnections);

        RoutingDecision {
            target: best_region,
            strategy: RoutingStrategy::LeastConnections,
            estimated_latency: 50,
            is_fallback: min_connections == u64::MAX,
            metadata: HashMap::new(),
        }
    }

    /// Weighted random routing
    fn route_weighted_random(&self, _request: &RoutingRequest) -> RoutingDecision {
        // Weight by available capacity
        let healthy: Vec<_> = self
            .locations
            .iter()
            .filter(|(_, loc)| loc.can_accept())
            .collect();

        if healthy.is_empty() {
            return self.fallback_decision();
        }

        let total_capacity: u32 = healthy.iter().map(|(_, loc)| loc.capacity).sum();
        let random_point = (simple_random() % total_capacity as u64) as u32;

        let mut cumulative = 0u32;
        for (region, location) in &healthy {
            cumulative += location.capacity;
            if random_point < cumulative {
                self.record_strategy(RoutingStrategy::WeightedRandom);
                return RoutingDecision {
                    target: **region,
                    strategy: RoutingStrategy::WeightedRandom,
                    estimated_latency: 50,
                    is_fallback: false,
                    metadata: HashMap::new(),
                };
            }
        }

        self.fallback_decision()
    }

    /// Geographic affinity routing
    fn route_geo_affinity(&self, request: &RoutingRequest) -> RoutingDecision {
        // Try to keep user in their region
        if let Some(location) = self.locations.get(&request.client_region) {
            if location.can_accept() {
                self.record_strategy(RoutingStrategy::GeoAffinity);
                return RoutingDecision {
                    target: request.client_region,
                    strategy: RoutingStrategy::GeoAffinity,
                    estimated_latency: 1,
                    is_fallback: false,
                    metadata: HashMap::new(),
                };
            }
        }

        // Fall back to nearest
        self.route_nearest(request)
    }

    /// A/B test routing
    fn route_ab_test(&self, request: &RoutingRequest, test: &AbTestConfig) -> RoutingDecision {
        let hash = simple_hash(&request.client_id);
        let variant_a = (hash % 100) < test.variant_a_percent as u64;

        let target = if variant_a {
            test.variant_a_target
        } else {
            test.variant_b_target
        };

        let mut metadata = HashMap::new();
        metadata.insert("ab_test".to_string(), test.name.clone());
        metadata.insert(
            "variant".to_string(),
            if variant_a { "A" } else { "B" }.to_string(),
        );

        self.record_strategy(RoutingStrategy::AbTest);

        RoutingDecision {
            target,
            strategy: RoutingStrategy::AbTest,
            estimated_latency: request.client_region.latency_to(&target),
            is_fallback: false,
            metadata,
        }
    }

    /// Canary deployment routing
    fn route_canary(&self, request: &RoutingRequest, canary: &CanaryConfig) -> RoutingDecision {
        let hash = simple_hash(&request.client_id);
        let is_canary = (hash % 100) < canary.canary_percent as u64;

        let target = if is_canary {
            canary.canary_target
        } else {
            canary.production_target
        };

        let mut metadata = HashMap::new();
        metadata.insert("canary".to_string(), canary.name.clone());
        metadata.insert("is_canary".to_string(), is_canary.to_string());

        self.record_strategy(RoutingStrategy::Canary);

        RoutingDecision {
            target,
            strategy: RoutingStrategy::Canary,
            estimated_latency: request.client_region.latency_to(&target),
            is_fallback: false,
            metadata,
        }
    }

    /// Fallback decision when no healthy locations available
    fn fallback_decision(&self) -> RoutingDecision {
        self.stats.fallbacks.fetch_add(1, Ordering::Relaxed);
        RoutingDecision {
            target: Region::NaEast, // Default fallback
            strategy: RoutingStrategy::Nearest,
            estimated_latency: 100,
            is_fallback: true,
            metadata: HashMap::new(),
        }
    }

    /// Record strategy usage for stats
    fn record_strategy(&self, strategy: RoutingStrategy) {
        let idx = match strategy {
            RoutingStrategy::Nearest => 0,
            RoutingStrategy::RoundRobin => 1,
            RoutingStrategy::LeastConnections => 2,
            RoutingStrategy::WeightedRandom => 3,
            RoutingStrategy::AbTest => 4,
            RoutingStrategy::Canary => 5,
            RoutingStrategy::GeoAffinity => 6,
        };
        self.stats.decisions_by_strategy[idx].fetch_add(1, Ordering::Relaxed);
    }

    /// Register an A/B test
    pub fn register_ab_test(&mut self, test: AbTestConfig) {
        self.ab_tests.insert(test.name.clone(), test);
    }

    /// Register a canary deployment
    pub fn register_canary(&mut self, canary: CanaryConfig) {
        self.canary_configs.insert(canary.name.clone(), canary);
    }

    /// Get router statistics
    pub fn stats(&self) -> &RouterStats {
        &self.stats
    }
}

/// Routing request
#[derive(Debug, Clone)]
pub struct RoutingRequest {
    /// Client region (from IP geolocation)
    pub client_region: Region,
    /// Unique client identifier (for consistent hashing)
    pub client_id: String,
    /// Preferred strategy
    pub strategy: Option<RoutingStrategy>,
    /// A/B test to use
    pub ab_test: Option<String>,
    /// Canary deployment to use
    pub canary: Option<String>,
    /// Additional hints
    pub hints: HashMap<String, String>,
}

impl RoutingRequest {
    pub fn new(client_region: Region, client_id: impl Into<String>) -> Self {
        Self {
            client_region,
            client_id: client_id.into(),
            strategy: None,
            ab_test: None,
            canary: None,
            hints: HashMap::new(),
        }
    }
}

/// Simple hash function for consistent routing
fn simple_hash(s: &str) -> u64 {
    let mut hash = 5381u64;
    for byte in s.bytes() {
        hash = hash.wrapping_mul(33).wrapping_add(byte as u64);
    }
    hash
}

/// Simple random number (not cryptographically secure)
fn simple_random() -> u64 {
    use std::time::SystemTime;
    SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap()
        .subsec_nanos() as u64
}

/// Request coalescer for deduplicating concurrent identical requests
pub struct RequestCoalescer {
    /// Pending requests by key
    pending: std::sync::RwLock<HashMap<String, PendingRequest>>,
    /// Coalescing window
    window: Duration,
    /// Statistics
    stats: CoalescerStats,
}

struct PendingRequest {
    /// When the first request arrived
    started_at: Instant,
    /// Number of coalesced requests
    count: AtomicUsize,
    /// Completion channel (would be proper async in real impl)
    completed: AtomicUsize,
}

/// Coalescer statistics
#[derive(Debug, Default)]
pub struct CoalescerStats {
    /// Total requests received
    pub total_requests: AtomicU64,
    /// Requests coalesced
    pub coalesced_requests: AtomicU64,
    /// Unique requests executed
    pub unique_executions: AtomicU64,
}

impl RequestCoalescer {
    pub fn new(window: Duration) -> Self {
        Self {
            pending: std::sync::RwLock::new(HashMap::new()),
            window,
            stats: CoalescerStats::default(),
        }
    }

    /// Try to coalesce a request
    pub fn try_coalesce(&self, key: &str) -> CoalesceResult {
        self.stats.total_requests.fetch_add(1, Ordering::Relaxed);

        let mut pending = self.pending.write().unwrap();

        if let Some(existing) = pending.get(key) {
            if existing.started_at.elapsed() < self.window {
                existing.count.fetch_add(1, Ordering::Relaxed);
                self.stats.coalesced_requests.fetch_add(1, Ordering::Relaxed);
                return CoalesceResult::Coalesced;
            }
        }

        // New request
        pending.insert(
            key.to_string(),
            PendingRequest {
                started_at: Instant::now(),
                count: AtomicUsize::new(1),
                completed: AtomicUsize::new(0),
            },
        );
        self.stats.unique_executions.fetch_add(1, Ordering::Relaxed);

        CoalesceResult::Execute
    }

    /// Mark request as complete
    pub fn complete(&self, key: &str) {
        let mut pending = self.pending.write().unwrap();
        pending.remove(key);
    }

    /// Get coalescing efficiency
    pub fn efficiency(&self) -> f64 {
        let total = self.stats.total_requests.load(Ordering::Relaxed);
        let unique = self.stats.unique_executions.load(Ordering::Relaxed);
        if total == 0 {
            return 0.0;
        }
        ((total - unique) as f64 / total as f64) * 100.0
    }
}

/// Result of coalescing attempt
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CoalesceResult {
    /// Should execute the request
    Execute,
    /// Request was coalesced with existing
    Coalesced,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_region_from_country() {
        assert_eq!(Region::from_country_code("US"), Region::NaEast);
        assert_eq!(Region::from_country_code("JP"), Region::ApNortheast);
        assert_eq!(Region::from_country_code("DE"), Region::EuCentral);
    }

    #[test]
    fn test_region_latency() {
        let na_east = Region::NaEast;
        assert_eq!(na_east.latency_to(&Region::NaEast), 1);
        assert!(na_east.latency_to(&Region::EuWest) > 1);
    }

    #[test]
    fn test_edge_location() {
        let location = EdgeLocation::new(Region::NaEast, 1000);
        assert!(location.can_accept());

        location.record_request();
        assert_eq!(location.connections.load(Ordering::Relaxed), 1);

        location.complete_request();
        assert_eq!(location.connections.load(Ordering::Relaxed), 0);
    }

    #[test]
    fn test_router_nearest() {
        let mut router = EdgeRouter::new(RoutingConfig::default());
        router.register_location(EdgeLocation::new(Region::NaEast, 1000));
        router.register_location(EdgeLocation::new(Region::EuWest, 1000));

        let request = RoutingRequest::new(Region::NaEast, "client1");
        let decision = router.route(&request);

        assert_eq!(decision.target, Region::NaEast);
        assert!(!decision.is_fallback);
    }

    #[test]
    fn test_router_round_robin() {
        let mut router = EdgeRouter::new(RoutingConfig {
            default_strategy: RoutingStrategy::RoundRobin,
            ..Default::default()
        });

        router.register_location(EdgeLocation::new(Region::NaEast, 1000));
        router.register_location(EdgeLocation::new(Region::NaWest, 1000));

        let request1 = RoutingRequest::new(Region::NaEast, "client1");
        let request2 = RoutingRequest::new(Region::NaEast, "client2");

        let decision1 = router.route(&request1);
        let decision2 = router.route(&request2);

        // Should route to different locations
        assert_ne!(decision1.target, decision2.target);
    }

    #[test]
    fn test_coalescer() {
        let coalescer = RequestCoalescer::new(Duration::from_millis(100));

        let result1 = coalescer.try_coalesce("key1");
        assert_eq!(result1, CoalesceResult::Execute);

        let result2 = coalescer.try_coalesce("key1");
        assert_eq!(result2, CoalesceResult::Coalesced);

        coalescer.complete("key1");

        let result3 = coalescer.try_coalesce("key1");
        assert_eq!(result3, CoalesceResult::Execute);
    }

    #[test]
    fn test_ab_test_routing() {
        let mut router = EdgeRouter::new(RoutingConfig::default());
        router.register_location(EdgeLocation::new(Region::NaEast, 1000));
        router.register_location(EdgeLocation::new(Region::NaWest, 1000));

        router.register_ab_test(AbTestConfig {
            name: "test1".to_string(),
            variant_a_percent: 50,
            variant_a_target: Region::NaEast,
            variant_b_target: Region::NaWest,
            active: true,
        });

        let mut request = RoutingRequest::new(Region::EuWest, "client1");
        request.ab_test = Some("test1".to_string());

        let decision = router.route(&request);
        assert!(decision.metadata.contains_key("ab_test"));
        assert!(decision.metadata.contains_key("variant"));
    }
}
