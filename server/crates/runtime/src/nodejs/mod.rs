//! Node.js runtime implementation
//!
//! This module provides a Node.js-based function runtime that implements the Runtime trait.
//! It supports both ES modules and CommonJS, with process pooling for warm starts.

pub mod executor;
pub mod process;

pub use executor::NodeJSExecutor;
pub use process::NodeProcess;

use std::path::PathBuf;
use std::process::Command;
use thiserror::Error;

/// Node.js-specific errors
#[derive(Error, Debug)]
pub enum NodeError {
    #[error("Node.js not found. Please install Node.js 18.x or later")]
    NodeNotFound,

    #[error("Node.js version {0} is not supported. Minimum version is 18.0.0")]
    UnsupportedVersion(String),

    #[error("Failed to spawn Node.js process: {0}")]
    SpawnFailed(String),

    #[error("Invalid JavaScript code: {0}")]
    InvalidCode(String),

    #[error("Handler function not found or not exported")]
    HandlerNotFound,

    #[error("IPC communication error: {0}")]
    IpcError(String),

    #[error("Process error: {0}")]
    ProcessError(String),
}

/// Node.js version information
#[derive(Debug, Clone)]
pub struct NodeVersion {
    pub major: u32,
    pub minor: u32,
    pub patch: u32,
    pub full: String,
}

impl NodeVersion {
    /// Parse Node.js version from string (e.g., "v18.16.0")
    pub fn parse(version_str: &str) -> Result<Self, NodeError> {
        let version_str = version_str.trim().trim_start_matches('v');
        let parts: Vec<&str> = version_str.split('.').collect();

        if parts.len() != 3 {
            return Err(NodeError::UnsupportedVersion(version_str.to_string()));
        }

        let major = parts[0]
            .parse()
            .map_err(|_| NodeError::UnsupportedVersion(version_str.to_string()))?;
        let minor = parts[1]
            .parse()
            .map_err(|_| NodeError::UnsupportedVersion(version_str.to_string()))?;
        let patch = parts[2]
            .parse()
            .map_err(|_| NodeError::UnsupportedVersion(version_str.to_string()))?;

        Ok(NodeVersion {
            major,
            minor,
            patch,
            full: format!("v{}.{}.{}", major, minor, patch),
        })
    }

    /// Check if this version meets minimum requirements (18.0.0)
    pub fn is_supported(&self) -> bool {
        self.major >= 18
    }
}

/// Detect Node.js installation
pub fn detect_nodejs() -> Result<(PathBuf, NodeVersion), NodeError> {
    // Try 'node' first
    if let Ok((path, version)) = try_node_binary("node") {
        return Ok((path, version));
    }

    // Try 'nodejs' as fallback (some Linux distributions)
    if let Ok((path, version)) = try_node_binary("nodejs") {
        return Ok((path, version));
    }

    Err(NodeError::NodeNotFound)
}

/// Try to execute a specific Node.js binary
fn try_node_binary(binary_name: &str) -> Result<(PathBuf, NodeVersion), NodeError> {
    let output = Command::new(binary_name)
        .arg("--version")
        .output()
        .map_err(|_| NodeError::NodeNotFound)?;

    if !output.status.success() {
        return Err(NodeError::NodeNotFound);
    }

    let version_str = String::from_utf8_lossy(&output.stdout);
    let version = NodeVersion::parse(&version_str)?;

    if !version.is_supported() {
        return Err(NodeError::UnsupportedVersion(version.full.clone()));
    }

    // Get the full path to the binary
    let which_output = Command::new("which")
        .arg(binary_name)
        .output()
        .map_err(|_| NodeError::NodeNotFound)?;

    let path_str = String::from_utf8_lossy(&which_output.stdout);
    let path = PathBuf::from(path_str.trim());

    Ok((path, version))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_version_parsing() {
        let version = NodeVersion::parse("v18.16.0").unwrap();
        assert_eq!(version.major, 18);
        assert_eq!(version.minor, 16);
        assert_eq!(version.patch, 0);
        assert_eq!(version.full, "v18.16.0");
    }

    #[test]
    fn test_version_parsing_without_v() {
        let version = NodeVersion::parse("20.5.1").unwrap();
        assert_eq!(version.major, 20);
        assert_eq!(version.minor, 5);
        assert_eq!(version.patch, 1);
    }

    #[test]
    fn test_version_support() {
        let v18 = NodeVersion::parse("v18.0.0").unwrap();
        assert!(v18.is_supported());

        let v20 = NodeVersion::parse("v20.5.1").unwrap();
        assert!(v20.is_supported());

        let v16 = NodeVersion {
            major: 16,
            minor: 20,
            patch: 0,
            full: "v16.20.0".to_string(),
        };
        assert!(!v16.is_supported());
    }

    #[test]
    fn test_invalid_version() {
        let result = NodeVersion::parse("invalid");
        assert!(result.is_err());
    }

    #[test]
    fn test_node_detection() {
        // This test will pass if Node.js is installed
        // Skip if Node.js is not available
        match detect_nodejs() {
            Ok((path, version)) => {
                println!("Found Node.js at {:?}", path);
                println!("Version: {}", version.full);
                assert!(version.is_supported());
            }
            Err(NodeError::NodeNotFound) => {
                println!("Node.js not installed - skipping test");
            }
            Err(e) => {
                panic!("Unexpected error: {}", e);
            }
        }
    }
}
