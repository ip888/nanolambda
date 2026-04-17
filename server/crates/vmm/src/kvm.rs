//! KVM (Kernel-based Virtual Machine) interface
//!
//! This module provides safe Rust wrappers around the KVM ioctl interface.
//! We use raw system calls rather than external crates for:
//! 1. Minimal dependencies (security)
//! 2. Full control over the interface
//! 3. Better understanding of the hypervisor interaction

use crate::error::{VmmError, VmmResult};
use std::fs::{File, OpenOptions};
use std::os::unix::io::{AsRawFd, RawFd};

/// KVM system file descriptor wrapper
pub struct Kvm {
    fd: File,
    version: KvmVersion,
    capabilities: KvmCapabilities,
}

/// KVM API version information
#[derive(Debug, Clone)]
pub struct KvmVersion {
    pub api_version: i32,
}

/// Available KVM capabilities
#[derive(Debug, Clone, Default)]
pub struct KvmCapabilities {
    pub user_memory: bool,
    pub irqchip: bool,
    pub ioapic: bool,
    pub pit2: bool,
    pub set_tss_addr: bool,
    pub ext_cpuid: bool,
    pub clocksource: bool,
    pub pit_state2: bool,
    pub irqfd: bool,
    pub ioeventfd: bool,
    pub vcpu_events: bool,
    pub sync_mmu: bool,
}

/// Capability identifiers for KVM_CHECK_EXTENSION
#[repr(u32)]
#[derive(Debug, Clone, Copy)]
pub enum KvmCapability {
    UserMemory = 3,
    Irqchip = 0,
    Ioapic = 26,
    Pit2 = 28,
    SetTssAddr = 4,
    ExtCpuid = 7,
    Clocksource = 18,
    PitState2 = 35,
    Irqfd = 32,
    Ioeventfd = 36,
    VcpuEvents = 41,
    SyncMmu = 16,
}

// KVM ioctl numbers (from linux/kvm.h)
const KVM_GET_API_VERSION: u64 = 0xAE00;
const KVM_CREATE_VM: u64 = 0xAE01;
const KVM_CHECK_EXTENSION: u64 = 0xAE03;
const KVM_GET_VCPU_MMAP_SIZE: u64 = 0xAE04;
const KVM_CREATE_VCPU: u64 = 0xAE41;
const KVM_SET_USER_MEMORY_REGION: u64 = 0x4020_AE46;
const KVM_RUN: u64 = 0xAE80;

impl Kvm {
    /// Check if KVM is available on this system
    pub fn check_available() -> VmmResult<bool> {
        match OpenOptions::new().read(true).write(true).open("/dev/kvm") {
            Ok(_) => Ok(true),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(false),
            Err(e) if e.kind() == std::io::ErrorKind::PermissionDenied => {
                Err(VmmError::KvmPermissionDenied)
            }
            Err(e) => Err(VmmError::KvmIoError(e)),
        }
    }

    /// Open the KVM device and initialize
    pub fn new() -> VmmResult<Self> {
        let fd = OpenOptions::new()
            .read(true)
            .write(true)
            .open("/dev/kvm")
            .map_err(|e| match e.kind() {
                std::io::ErrorKind::NotFound => VmmError::KvmNotAvailable,
                std::io::ErrorKind::PermissionDenied => VmmError::KvmPermissionDenied,
                _ => VmmError::KvmIoError(e),
            })?;

        // Get API version
        let api_version = unsafe { libc::ioctl(fd.as_raw_fd(), KVM_GET_API_VERSION, 0) };
        if api_version < 0 {
            return Err(VmmError::KvmIoctlFailed("KVM_GET_API_VERSION"));
        }

        // We require KVM API version 12 (stable since Linux 2.6.x)
        if api_version != 12 {
            return Err(VmmError::KvmUnsupportedVersion(api_version));
        }

        let version = KvmVersion { api_version };

        // Probe capabilities
        let kvm_fd = fd.as_raw_fd();
        let capabilities = KvmCapabilities {
            user_memory: Self::check_cap(kvm_fd, KvmCapability::UserMemory),
            irqchip: Self::check_cap(kvm_fd, KvmCapability::Irqchip),
            ioapic: Self::check_cap(kvm_fd, KvmCapability::Ioapic),
            pit2: Self::check_cap(kvm_fd, KvmCapability::Pit2),
            set_tss_addr: Self::check_cap(kvm_fd, KvmCapability::SetTssAddr),
            ext_cpuid: Self::check_cap(kvm_fd, KvmCapability::ExtCpuid),
            clocksource: Self::check_cap(kvm_fd, KvmCapability::Clocksource),
            pit_state2: Self::check_cap(kvm_fd, KvmCapability::PitState2),
            irqfd: Self::check_cap(kvm_fd, KvmCapability::Irqfd),
            ioeventfd: Self::check_cap(kvm_fd, KvmCapability::Ioeventfd),
            vcpu_events: Self::check_cap(kvm_fd, KvmCapability::VcpuEvents),
            sync_mmu: Self::check_cap(kvm_fd, KvmCapability::SyncMmu),
        };

        // Verify required capabilities
        if !capabilities.user_memory {
            return Err(VmmError::KvmMissingCapability("USER_MEMORY"));
        }

        Ok(Self {
            fd,
            version,
            capabilities,
        })
    }

    /// Check if a specific capability is available
    fn check_cap(fd: RawFd, cap: KvmCapability) -> bool {
        let ret = unsafe { libc::ioctl(fd, KVM_CHECK_EXTENSION, cap as u32) };
        ret > 0
    }

    /// Get the KVM version
    pub fn version(&self) -> &KvmVersion {
        &self.version
    }

    /// Get the capabilities
    pub fn capabilities(&self) -> &KvmCapabilities {
        &self.capabilities
    }

    /// Get the maximum number of vCPUs supported
    pub fn max_vcpus(&self) -> u32 {
        let ret = unsafe { libc::ioctl(self.fd.as_raw_fd(), KVM_CHECK_EXTENSION, 9u32) }; // KVM_CAP_NR_VCPUS
        if ret > 0 {
            ret as u32
        } else {
            4 // Conservative default
        }
    }

    /// Get the maximum number of memory slots
    pub fn max_memory_slots(&self) -> u32 {
        let ret = unsafe { libc::ioctl(self.fd.as_raw_fd(), KVM_CHECK_EXTENSION, 10u32) }; // KVM_CAP_NR_MEMSLOTS
        if ret > 0 {
            ret as u32
        } else {
            32 // Conservative default
        }
    }

    /// Get the vCPU mmap size (for KVM_RUN)
    pub fn vcpu_mmap_size(&self) -> VmmResult<usize> {
        let ret = unsafe { libc::ioctl(self.fd.as_raw_fd(), KVM_GET_VCPU_MMAP_SIZE, 0) };
        if ret < 0 {
            return Err(VmmError::KvmIoctlFailed("KVM_GET_VCPU_MMAP_SIZE"));
        }
        Ok(ret as usize)
    }

    /// Create a new VM
    pub fn create_vm(&self) -> VmmResult<RawFd> {
        let ret = unsafe { libc::ioctl(self.fd.as_raw_fd(), KVM_CREATE_VM, 0) };
        if ret < 0 {
            return Err(VmmError::KvmIoctlFailed("KVM_CREATE_VM"));
        }
        Ok(ret)
    }

    /// Get the raw file descriptor
    pub fn as_raw_fd(&self) -> RawFd {
        self.fd.as_raw_fd()
    }
}

impl Drop for Kvm {
    fn drop(&mut self) {
        // File is automatically closed when dropped
        tracing::debug!("KVM device closed");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_kvm_availability_check() {
        // Should not panic
        let _ = Kvm::check_available();
    }
}
