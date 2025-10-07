//! Virtual Machine management
//!
//! This module handles the creation, configuration, and lifecycle of microVMs.

use kvm_ioctls::{Kvm, VmFd};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use tracing::{debug, info};

use crate::{Result, VmmError};

/// VM configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VmConfig {
    /// Number of vCPUs (1-2 recommended)
    pub vcpu_count: u8,
    
    /// Memory size in MB
    pub memory_mb: u32,
    
    /// Path to kernel image
    pub kernel_path: PathBuf,
    
    /// Path to root filesystem
    pub rootfs_path: Option<PathBuf>,
    
    /// Kernel command line parameters
    pub kernel_cmdline: String,
}

impl Default for VmConfig {
    fn default() -> Self {
        Self {
            vcpu_count: 1,
            memory_mb: 128,
            kernel_path: PathBuf::from("/boot/vmlinux"),
            rootfs_path: None,
            kernel_cmdline: String::from("console=ttyS0 reboot=k panic=1"),
        }
    }
}

/// VM state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VmState {
    Created,
    Running,
    Paused,
    Stopped,
}

/// MicroVM instance
pub struct Vm {
    config: VmConfig,
    vm_fd: VmFd,
    state: VmState,
}

impl Vm {
    /// Create a new VM (not started)
    pub fn new(config: VmConfig) -> Result<Self> {
        info!("Creating new VM with config: {:?}", config);
        
        // Open /dev/kvm
        let kvm = Kvm::new().map_err(|e| {
            VmmError::InvalidConfig(format!("Failed to open /dev/kvm: {}. Is KVM available?", e))
        })?;
        
        // Create VM
        let vm_fd = kvm.create_vm().map_err(|e| {
            VmmError::Kvm(e)
        })?;
        
        debug!("VM file descriptor created");
        
        Ok(Self {
            config,
            vm_fd,
            state: VmState::Created,
        })
    }
    
    /// Get VM configuration
    pub fn config(&self) -> &VmConfig {
        &self.config
    }
    
    /// Get current VM state
    pub fn state(&self) -> VmState {
        self.state
    }
    
    /// Start the VM
    pub fn start(&mut self) -> Result<()> {
        info!("Starting VM");
        
        if self.state != VmState::Created {
            return Err(VmmError::InvalidConfig(
                format!("Cannot start VM in state {:?}", self.state)
            ));
        }
        
        // TODO: Initialize vCPUs and start execution
        self.state = VmState::Running;
        
        Ok(())
    }
    
    /// Pause the VM
    pub fn pause(&mut self) -> Result<()> {
        info!("Pausing VM");
        
        if self.state != VmState::Running {
            return Err(VmmError::InvalidConfig(
                format!("Cannot pause VM in state {:?}", self.state)
            ));
        }
        
        // TODO: Pause vCPU execution
        self.state = VmState::Paused;
        
        Ok(())
    }
    
    /// Resume the VM
    pub fn resume(&mut self) -> Result<()> {
        info!("Resuming VM");
        
        if self.state != VmState::Paused {
            return Err(VmmError::InvalidConfig(
                format!("Cannot resume VM in state {:?}", self.state)
            ));
        }
        
        self.state = VmState::Running;
        
        Ok(())
    }
    
    /// Stop the VM
    pub fn stop(&mut self) -> Result<()> {
        info!("Stopping VM");
        
        self.state = VmState::Stopped;
        
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_vm_config_default() {
        let config = VmConfig::default();
        assert_eq!(config.vcpu_count, 1);
        assert_eq!(config.memory_mb, 128);
    }
    
    #[test]
    fn test_vm_state_transitions() {
        // This test will only work on systems with KVM
        // For now, it's a placeholder
        assert_eq!(VmState::Created, VmState::Created);
    }
}
