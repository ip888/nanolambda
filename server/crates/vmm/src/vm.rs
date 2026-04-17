//! VM instance management

use crate::config::VmConfig;
use crate::cpu::{VCpu, VCpuState};
use crate::error::{VmmError, VmmResult};
use crate::kvm::Kvm;
use crate::memory::GuestMemory;
use std::os::unix::io::RawFd;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

/// VM state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VmState {
    /// VM created but not started
    Created,
    /// VM is running
    Running,
    /// VM is paused
    Paused,
    /// VM has stopped
    Stopped,
}

/// A virtual machine instance
#[derive(Clone)]
pub struct VmInstance {
    inner: Arc<VmInstanceInner>,
}

struct VmInstanceInner {
    /// VM file descriptor
    vm_fd: RawFd,
    /// Configuration
    config: VmConfig,
    /// Guest memory
    memory: std::sync::RwLock<GuestMemory>,
    /// KVM reference
    kvm: Arc<Kvm>,
    /// Running flag
    running: AtomicBool,
    /// VM state
    state: std::sync::RwLock<VmState>,
}

impl VmInstance {
    /// Create a new VM instance
    pub fn new(kvm: &Kvm, config: VmConfig) -> VmmResult<Self> {
        config.validate()?;

        let vm_fd = kvm.create_vm()?;
        let kvm = Arc::new(Kvm::new()?);

        // Initialize guest memory
        let mut memory = GuestMemory::new(vm_fd);

        // Add main memory region at address 0
        let memory_bytes = config.memory_mb * 1024 * 1024;
        memory.add_region(0, memory_bytes, false)?;

        tracing::info!(
            "Created VM: {} MB memory, {} vCPUs",
            config.memory_mb,
            config.vcpus
        );

        Ok(Self {
            inner: Arc::new(VmInstanceInner {
                vm_fd,
                config,
                memory: std::sync::RwLock::new(memory),
                kvm,
                running: AtomicBool::new(false),
                state: std::sync::RwLock::new(VmState::Created),
            }),
        })
    }

    /// Get the VM configuration
    pub fn config(&self) -> &VmConfig {
        &self.inner.config
    }

    /// Check if the VM is running
    pub fn is_running(&self) -> bool {
        self.inner.running.load(Ordering::SeqCst)
    }

    /// Get current VM state
    pub fn state(&self) -> VmState {
        *self.inner.state.read().expect("State lock should not be poisoned")
    }

    /// Start the VM
    pub fn start(&self) -> VmmResult<()> {
        if self.is_running() {
            return Err(VmmError::VmAlreadyRunning);
        }

        // Verify kernel is loaded
        if self.inner.config.kernel_path.is_none() {
            return Err(VmmError::BootError(
                "No kernel path configured".to_string(),
            ));
        }

        self.inner.running.store(true, Ordering::SeqCst);
        *self.inner.state.write().expect("State lock should not be poisoned during start") = VmState::Running;

        tracing::info!("VM started");
        Ok(())
    }

    /// Pause the VM
    pub fn pause(&self) -> VmmResult<()> {
        if !self.is_running() {
            return Err(VmmError::VmNotRunning);
        }

        *self.inner.state.write().expect("State lock should not be poisoned during pause") = VmState::Paused;
        tracing::info!("VM paused");
        Ok(())
    }

    /// Resume the VM
    pub fn resume(&self) -> VmmResult<()> {
        let state = self.state();
        if state != VmState::Paused {
            return Err(VmmError::InvalidConfig(
                "VM is not paused".to_string(),
            ));
        }

        *self.inner.state.write().expect("State lock should not be poisoned during resume") = VmState::Running;
        tracing::info!("VM resumed");
        Ok(())
    }

    /// Stop the VM
    pub fn stop(&self) -> VmmResult<()> {
        self.inner.running.store(false, Ordering::SeqCst);
        *self.inner.state.write().expect("State lock should not be poisoned during stop") = VmState::Stopped;
        tracing::info!("VM stopped");
        Ok(())
    }

    /// Write data to guest memory
    pub fn write_memory(&self, guest_addr: u64, data: &[u8]) -> VmmResult<()> {
        self.inner
            .memory
            .read()
            .expect("Memory lock should not be poisoned")
            .write_at(guest_addr, data)
    }

    /// Read data from guest memory
    pub fn read_memory(&self, guest_addr: u64, size: usize) -> VmmResult<Vec<u8>> {
        self.inner
            .memory
            .read()
            .expect("Memory lock should not be poisoned")
            .read_at(guest_addr, size)
    }

    /// Get the VM file descriptor
    pub fn vm_fd(&self) -> RawFd {
        self.inner.vm_fd
    }

    /// Get KVM reference
    pub fn kvm(&self) -> &Arc<Kvm> {
        &self.inner.kvm
    }

    /// Create a vCPU
    pub fn create_vcpu(&self, id: u32) -> VmmResult<VCpu> {
        VCpu::new(self.inner.vm_fd, Arc::clone(&self.inner.kvm), id)
    }

    /// Get total memory size
    pub fn memory_size(&self) -> u64 {
        self.inner
            .memory
            .read()
            .expect("Memory lock should not be poisoned")
            .total_size()
    }
}

impl Drop for VmInstanceInner {
    fn drop(&mut self) {
        unsafe {
            libc::close(self.vm_fd);
        }
        tracing::debug!("VM instance destroyed");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_vm_state_transitions() {
        assert_eq!(VmState::Created, VmState::Created);
        assert_ne!(VmState::Created, VmState::Running);
    }
}
