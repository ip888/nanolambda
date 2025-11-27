//! Memory management for POC
//!
//! Simplified memory management focused on basic functionality.

use vm_memory::{
    GuestMemoryMmap, GuestAddress,
    MmapRegion, GuestRegionMmap,
    Bytes
};
use tracing::{info, debug, error};

use crate::Result;

/// Memory configuration
#[derive(Debug, Clone, Copy)]
pub struct MemoryConfig {
    /// Size in megabytes
    pub size_mb: u64,
    /// Start address in guest physical memory
    pub start_address: u64,
}

impl Default for MemoryConfig {
    fn default() -> Self {
        Self {
            size_mb: 128,
            start_address: 0,
        }
    }
}

/// Memory manager for guest VM
pub struct MemoryManager {
    config: MemoryConfig,
    memory: GuestMemoryMmap,
}

impl MemoryManager {
    /// Create a new memory manager with the given configuration
    pub fn new(config: MemoryConfig) -> Result<Self> {
        info!("Initializing memory manager with {}MB", config.size_mb);
        
        let mem_size = config.size_mb * 1024 * 1024;
        
        // Create mmap region
        let mmap_region = MmapRegion::new(mem_size as usize)
            .map_err(|e| {
                error!("Failed to create mmap region: {}", e);
                crate::VmmError::InvalidConfig(format!("Mmap region creation failed: {}", e))
            })?;
        
        // Create guest region (returns Result in vm-memory 0.16)
        let region = GuestRegionMmap::new(mmap_region, GuestAddress(config.start_address))
            .map_err(|e| {
                error!("Failed to create guest region: {}", e);
                crate::VmmError::InvalidConfig(format!("Guest region creation failed: {}", e))
            })?;

        // Create guest memory from regions
        let memory = GuestMemoryMmap::from_regions(vec![region])
            .map_err(|e| {
                error!("Failed to create guest memory: {}", e);
                crate::VmmError::InvalidConfig(format!("Guest memory creation failed: {}", e))
            })?;

        debug!("Successfully allocated {}MB of guest memory", config.size_mb);

        Ok(Self {
            config,
            memory,
        })
    }

    /// Get a reference to the guest memory
    pub fn memory(&self) -> &GuestMemoryMmap {
        &self.memory
    }

    /// Write a slice to guest memory at the specified address
    pub fn write_slice(&self, data: &[u8], addr: GuestAddress) -> Result<()> {
        self.memory.write_slice(data, addr).map_err(|e| {
            error!("Failed to write to guest memory at {:?}: {}", addr, e);
            e.into()
        })
    }

    /// Read a slice from guest memory at the specified address
    pub fn read_slice(&self, data: &mut [u8], addr: GuestAddress) -> Result<()> {
        self.memory.read_slice(data, addr).map_err(|e| {
            error!("Failed to read from guest memory at {:?}: {}", addr, e);
            e.into()
        })
    }

    /// Get the size of allocated memory in bytes
    pub fn size(&self) -> u64 {
        self.config.size_mb * 1024 * 1024
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_memory_manager_creation() {
        let config = MemoryConfig::default();
        let mm = MemoryManager::new(config);
        assert!(mm.is_ok(), "Memory manager creation should succeed");
    }

    #[test]
    fn test_memory_write_read() {
        let mm = MemoryManager::new(MemoryConfig::default()).unwrap();
        
        let test_data = b"Hello, VM!";
        let addr = GuestAddress(0x1000);
        
        mm.write_slice(test_data, addr).unwrap();
        
        let mut read_buffer = vec![0u8; test_data.len()];
        mm.read_slice(&mut read_buffer, addr).unwrap();
        
        assert_eq!(&read_buffer, test_data);
    }
}