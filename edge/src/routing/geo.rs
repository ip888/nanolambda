//! Geo-Aware Routing for Edge Functions
//!
//! Smart request routing based on geographic location:
//! - Latency-based routing
//! - Geographic restrictions
//! - Regional failover
//! - Data residency compliance

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use worker::{Request, Cf};

/// Geographic region codes
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Region {
    /// North America East
    NaEast,
    /// North America West
    NaWest,
    /// Europe West
    EuWest,
    /// Europe Central
    EuCentral,
    /// Asia Pacific North (Japan, Korea)
    ApacNorth,
    /// Asia Pacific South (Singapore, Australia)
    ApacSouth,
    /// South America
    SaEast,
    /// Middle East
    MeWest,
    /// Africa
    AfSouth,
    /// Unknown/Global
    Global,
}

impl Region {
    /// Get region from country code
    pub fn from_country(country: &str) -> Self {
        match country.to_uppercase().as_str() {
            // North America
            "US" | "CA" | "MX" => {
                // Further split by timezone/location would need more info
                Self::NaEast
            }
            // Europe West
            "GB" | "IE" | "FR" | "ES" | "PT" | "NL" | "BE" => Self::EuWest,
            // Europe Central
            "DE" | "AT" | "CH" | "PL" | "CZ" | "HU" | "IT" => Self::EuCentral,
            // APAC North
            "JP" | "KR" | "TW" => Self::ApacNorth,
            // APAC South
            "SG" | "AU" | "NZ" | "ID" | "MY" | "TH" | "VN" | "PH" | "IN" => Self::ApacSouth,
            // South America
            "BR" | "AR" | "CL" | "CO" | "PE" => Self::SaEast,
            // Middle East
            "AE" | "SA" | "IL" | "TR" | "EG" => Self::MeWest,
            // Africa
            "ZA" | "NG" | "KE" | "GH" => Self::AfSouth,
            _ => Self::Global,
        }
    }

    /// Get region display name
    pub fn display_name(&self) -> &'static str {
        match self {
            Self::NaEast => "North America (East)",
            Self::NaWest => "North America (West)",
            Self::EuWest => "Europe (West)",
            Self::EuCentral => "Europe (Central)",
            Self::ApacNorth => "Asia Pacific (North)",
            Self::ApacSouth => "Asia Pacific (South)",
            Self::SaEast => "South America (East)",
            Self::MeWest => "Middle East (West)",
            Self::AfSouth => "Africa (South)",
            Self::Global => "Global",
        }
    }
}

/// Geographic location info extracted from request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GeoLocation {
    /// Country code (ISO 3166-1 alpha-2)
    pub country: Option<String>,
    /// Region/state/province
    pub region: Option<String>,
    /// City name
    pub city: Option<String>,
    /// Latitude
    pub latitude: Option<f64>,
    /// Longitude
    pub longitude: Option<f64>,
    /// Timezone
    pub timezone: Option<String>,
    /// ASN (Autonomous System Number)
    pub asn: Option<u32>,
    /// Continent
    pub continent: Option<String>,
    /// Is EU country
    pub is_eu: Option<bool>,
    /// Cloudflare colo (data center)
    pub colo: Option<String>,
}

impl GeoLocation {
    /// Extract geo info from Cloudflare request
    pub fn from_cf(cf: &Cf) -> Self {
        let (latitude, longitude) = cf.coordinates()
            .map(|(lat, lon)| (Some(lat as f64), Some(lon as f64)))
            .unwrap_or((None, None));
        
        Self {
            country: cf.country(),
            region: cf.region(),
            city: cf.city(),
            latitude,
            longitude,
            timezone: Some(cf.timezone_name()),
            asn: cf.asn(),
            continent: cf.continent(),
            is_eu: Some(cf.is_eu_country()),
            colo: Some(cf.colo()),
        }
    }

    /// Get the resolved region
    pub fn get_region(&self) -> Region {
        self.country
            .as_ref()
            .map(|c| Region::from_country(c))
            .unwrap_or(Region::Global)
    }

    /// Calculate approximate distance to another location (Haversine formula)
    pub fn distance_km(&self, other_lat: f64, other_lon: f64) -> Option<f64> {
        let lat1 = self.latitude?;
        let lon1 = self.longitude?;

        let r = 6371.0; // Earth radius in km
        let lat1_rad = lat1.to_radians();
        let lat2_rad = other_lat.to_radians();
        let delta_lat = (other_lat - lat1).to_radians();
        let delta_lon = (other_lon - lon1).to_radians();

        let a = (delta_lat / 2.0).sin().powi(2)
            + lat1_rad.cos() * lat2_rad.cos() * (delta_lon / 2.0).sin().powi(2);
        let c = 2.0 * a.sqrt().asin();

        Some(r * c)
    }
}

/// Geo routing configuration
#[derive(Debug, Clone)]
pub struct GeoRoutingConfig {
    /// Allowed regions (empty = all allowed)
    pub allowed_regions: Vec<Region>,
    /// Blocked regions
    pub blocked_regions: Vec<Region>,
    /// Blocked countries (ISO codes)
    pub blocked_countries: Vec<String>,
    /// GDPR mode (restrict non-EU data access)
    pub gdpr_mode: bool,
    /// Data residency requirements (country -> required region)
    pub data_residency: HashMap<String, Region>,
    /// Regional endpoints
    pub regional_endpoints: HashMap<Region, String>,
    /// Enable latency-based routing
    pub latency_routing: bool,
}

impl Default for GeoRoutingConfig {
    fn default() -> Self {
        Self {
            allowed_regions: Vec::new(),
            blocked_regions: Vec::new(),
            blocked_countries: Vec::new(),
            gdpr_mode: false,
            data_residency: HashMap::new(),
            regional_endpoints: HashMap::new(),
            latency_routing: true,
        }
    }
}

/// Geo router for edge functions
pub struct GeoRouter {
    config: GeoRoutingConfig,
    /// Regional backend locations (region -> (lat, lon))
    backend_locations: HashMap<Region, (f64, f64)>,
}

impl GeoRouter {
    /// Create new geo router
    pub fn new(config: GeoRoutingConfig) -> Self {
        let mut backend_locations = HashMap::new();
        // Default backend locations (major cloud regions)
        backend_locations.insert(Region::NaEast, (39.0438, -77.4874)); // Virginia
        backend_locations.insert(Region::NaWest, (37.3861, -122.0839)); // California
        backend_locations.insert(Region::EuWest, (53.3498, -6.2603)); // Ireland
        backend_locations.insert(Region::EuCentral, (50.1109, 8.6821)); // Frankfurt
        backend_locations.insert(Region::ApacNorth, (35.6762, 139.6503)); // Tokyo
        backend_locations.insert(Region::ApacSouth, (1.3521, 103.8198)); // Singapore

        Self {
            config,
            backend_locations,
        }
    }

    /// Check if request is allowed based on geo rules
    pub fn is_allowed(&self, geo: &GeoLocation) -> GeoCheckResult {
        // Check blocked countries
        if let Some(ref country) = geo.country {
            if self.config.blocked_countries.contains(country) {
                return GeoCheckResult::Blocked(BlockReason::Country(country.clone()));
            }
        }

        // Check blocked regions
        let region = geo.get_region();
        if self.config.blocked_regions.contains(&region) {
            return GeoCheckResult::Blocked(BlockReason::Region(region));
        }

        // Check allowed regions (if configured)
        if !self.config.allowed_regions.is_empty()
            && !self.config.allowed_regions.contains(&region)
        {
            return GeoCheckResult::Blocked(BlockReason::RegionNotAllowed(region));
        }

        // GDPR check
        if self.config.gdpr_mode {
            if let Some(is_eu) = geo.is_eu {
                if is_eu {
                    // EU user - check data stays in EU
                    let resolved_region = self.resolve_region(geo);
                    if resolved_region != Region::EuWest && resolved_region != Region::EuCentral {
                        return GeoCheckResult::Redirect(Region::EuWest);
                    }
                }
            }
        }

        GeoCheckResult::Allowed
    }

    /// Resolve best region for request
    pub fn resolve_region(&self, geo: &GeoLocation) -> Region {
        if self.config.latency_routing {
            self.lowest_latency_region(geo)
        } else {
            geo.get_region()
        }
    }

    /// Find lowest latency region based on distance
    fn lowest_latency_region(&self, geo: &GeoLocation) -> Region {
        let mut best_region = Region::Global;
        let mut min_distance = f64::MAX;

        for (region, (lat, lon)) in &self.backend_locations {
            if let Some(distance) = geo.distance_km(*lat, *lon) {
                if distance < min_distance {
                    min_distance = distance;
                    best_region = *region;
                }
            }
        }

        best_region
    }

    /// Get endpoint for region
    pub fn get_endpoint(&self, region: Region) -> Option<&String> {
        self.config.regional_endpoints.get(&region)
    }

    /// Create routing decision
    pub fn route(&self, geo: &GeoLocation) -> RoutingDecision {
        let check = self.is_allowed(geo);

        match check {
            GeoCheckResult::Allowed => {
                let region = self.resolve_region(geo);
                let endpoint = self.get_endpoint(region).cloned();
                RoutingDecision {
                    allowed: true,
                    region,
                    endpoint,
                    redirect: None,
                    reason: None,
                }
            }
            GeoCheckResult::Blocked(reason) => RoutingDecision {
                allowed: false,
                region: Region::Global,
                endpoint: None,
                redirect: None,
                reason: Some(format!("{:?}", reason)),
            },
            GeoCheckResult::Redirect(target_region) => {
                let endpoint = self.get_endpoint(target_region).cloned();
                RoutingDecision {
                    allowed: true,
                    region: target_region,
                    endpoint,
                    redirect: Some(target_region),
                    reason: Some("Data residency requirement".to_string()),
                }
            }
        }
    }
}

/// Result of geo check
#[derive(Debug)]
pub enum GeoCheckResult {
    /// Request allowed
    Allowed,
    /// Request blocked
    Blocked(BlockReason),
    /// Request should be redirected to different region
    Redirect(Region),
}

/// Reason for blocking
#[derive(Debug)]
pub enum BlockReason {
    /// Blocked country
    Country(String),
    /// Blocked region
    Region(Region),
    /// Region not in allowed list
    RegionNotAllowed(Region),
}

/// Routing decision
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RoutingDecision {
    /// Is request allowed
    pub allowed: bool,
    /// Resolved region
    pub region: Region,
    /// Backend endpoint (if configured)
    pub endpoint: Option<String>,
    /// Redirect target (if needed)
    pub redirect: Option<Region>,
    /// Reason for decision
    pub reason: Option<String>,
}

/// Helper to extract geo from request
pub fn extract_geo(req: &Request) -> Option<GeoLocation> {
    req.cf().map(|cf| GeoLocation::from_cf(cf))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_region_from_country() {
        assert_eq!(Region::from_country("US"), Region::NaEast);
        assert_eq!(Region::from_country("GB"), Region::EuWest);
        assert_eq!(Region::from_country("DE"), Region::EuCentral);
        assert_eq!(Region::from_country("JP"), Region::ApacNorth);
        assert_eq!(Region::from_country("SG"), Region::ApacSouth);
        assert_eq!(Region::from_country("XX"), Region::Global);
    }

    #[test]
    fn test_distance_calculation() {
        let geo = GeoLocation {
            country: Some("US".to_string()),
            region: None,
            city: None,
            latitude: Some(40.7128), // NYC
            longitude: Some(-74.0060),
            timezone: None,
            asn: None,
            continent: None,
            is_eu: None,
            colo: None,
        };

        // Distance to London
        let distance = geo.distance_km(51.5074, -0.1278).unwrap();
        assert!(distance > 5500.0 && distance < 5700.0); // ~5570 km
    }

    #[test]
    fn test_geo_router_blocking() {
        let mut config = GeoRoutingConfig::default();
        config.blocked_countries.push("RU".to_string());

        let router = GeoRouter::new(config);

        let geo_blocked = GeoLocation {
            country: Some("RU".to_string()),
            region: None,
            city: None,
            latitude: None,
            longitude: None,
            timezone: None,
            asn: None,
            continent: None,
            is_eu: None,
            colo: None,
        };

        let result = router.is_allowed(&geo_blocked);
        assert!(matches!(result, GeoCheckResult::Blocked(_)));
    }

    #[test]
    fn test_routing_decision() {
        let mut config = GeoRoutingConfig::default();
        config
            .regional_endpoints
            .insert(Region::EuWest, "https://eu.api.example.com".to_string());

        let router = GeoRouter::new(config);

        let geo = GeoLocation {
            country: Some("GB".to_string()),
            region: None,
            city: None,
            latitude: Some(51.5074),
            longitude: Some(-0.1278),
            timezone: None,
            asn: None,
            continent: None,
            is_eu: Some(false),
            colo: None,
        };

        let decision = router.route(&geo);
        assert!(decision.allowed);
    }
}
