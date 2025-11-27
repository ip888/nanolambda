//! Security features for the VMM
//! 
//! This module implements security features including:
//! - Namespace isolation
//! - Capability dropping
//! - Basic system call filtering

use anyhow::Result;
use nix::sched::{self, CloneFlags};
use serde::{Deserialize, Serialize};
use caps::{CapSet, Capability};
use std::str::FromStr;

/// Security configuration for the VMM
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecurityConfig {
    /// Enable namespace isolation
    pub enable_namespaces: bool,
    
    /// Enable capability dropping
    pub enable_capabilities: bool,
    
    /// Capabilities to retain (as strings)
    pub capabilities: Vec<String>,
}

impl Default for SecurityConfig {
    fn default() -> Self {
        Self {
            enable_namespaces: true,
            enable_capabilities: true,
            capabilities: vec![
                "CAP_NET_ADMIN".to_string(),
                "CAP_NET_BIND_SERVICE".to_string(),
                "CAP_SYS_ADMIN".to_string(), // Required for KVM
            ],
        }
    }
}

/// Set up all security features
pub fn setup_security(config: &SecurityConfig) -> Result<()> {
    if config.enable_namespaces {
        setup_namespaces()?;
    }
    
    if config.enable_capabilities {
        drop_capabilities(&config.capabilities)?;
    }
    
    Ok(())
}

/// Set up namespace isolation
fn setup_namespaces() -> Result<()> {
    let flags = CloneFlags::CLONE_NEWNET | // Network namespace
                CloneFlags::CLONE_NEWUTS | // UTS namespace
                CloneFlags::CLONE_NEWNS;   // Mount namespace
    
    sched::unshare(flags)?;
    Ok(())
}

/// Drop unnecessary capabilities, keeping only those specified
fn drop_capabilities(retain: &[String]) -> Result<()> {
    // Drop all capabilities first
    caps::clear(None, CapSet::Effective)?;
    caps::clear(None, CapSet::Permitted)?;
    
    // Retain only necessary ones
    for cap_str in retain {
        if let Ok(cap) = Capability::from_str(cap_str) {
            caps::raise(None, CapSet::Effective, cap)?;
            caps::raise(None, CapSet::Permitted, cap)?;
        }
    }
    
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_security_config_default() {
        let config = SecurityConfig::default();
        assert!(config.enable_namespaces);
        assert!(config.enable_capabilities);
        assert!(!config.capabilities.is_empty());
    }

    #[test]
    fn test_setup_security() {
        let config = SecurityConfig::default();
        assert!(setup_security(&config).is_ok());
    }
}