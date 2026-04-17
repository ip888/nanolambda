//! Virtual CPU management

use crate::error::{VmmError, VmmResult};
use crate::kvm::Kvm;
use std::os::unix::io::RawFd;
use std::sync::Arc;

/// vCPU configuration
#[derive(Debug, Clone)]
pub struct VCpuConfig {
    /// vCPU index
    pub id: u32,
    /// Initial instruction pointer (for 64-bit mode)
    pub entry_point: u64,
}

/// vCPU state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VCpuState {
    /// vCPU is created but not started
    Created,
    /// vCPU is running guest code
    Running,
    /// vCPU exited due to I/O
    IoExit,
    /// vCPU exited due to MMIO
    MmioExit,
    /// vCPU halted (HLT instruction)
    Halted,
    /// vCPU shutdown
    Shutdown,
    /// vCPU error
    Error,
}

/// Virtual CPU instance
pub struct VCpu {
    /// vCPU file descriptor
    fd: RawFd,
    /// vCPU index
    id: u32,
    /// Reference to KVM for mmap size
    kvm: Arc<Kvm>,
    /// Current state
    state: VCpuState,
    /// KVM run structure mmap
    kvm_run: Option<*mut KvmRun>,
}

// KVM run structure exit reasons
const KVM_EXIT_UNKNOWN: u32 = 0;
const KVM_EXIT_IO: u32 = 2;
const KVM_EXIT_MMIO: u32 = 6;
const KVM_EXIT_HLT: u32 = 5;
const KVM_EXIT_SHUTDOWN: u32 = 8;
const KVM_EXIT_INTERNAL_ERROR: u32 = 17;

const KVM_RUN: u64 = 0xAE80;
const KVM_CREATE_VCPU: u64 = 0xAE41;

/// KVM run structure (simplified - actual structure is much larger)
#[repr(C)]
struct KvmRun {
    request_interrupt_window: u8,
    immediate_exit: u8,
    padding1: [u8; 6],
    exit_reason: u32,
    ready_for_interrupt_injection: u8,
    if_flag: u8,
    flags: u16,
    cr8: u64,
    apic_base: u64,
    // Union for exit data follows - we access it through raw pointers
}

impl VCpu {
    /// Create a new vCPU
    pub fn new(vm_fd: RawFd, kvm: Arc<Kvm>, id: u32) -> VmmResult<Self> {
        let fd = unsafe { libc::ioctl(vm_fd, KVM_CREATE_VCPU, id) };
        if fd < 0 {
            return Err(VmmError::VCpuCreationFailed(format!(
                "KVM_CREATE_VCPU failed for vCPU {}",
                id
            )));
        }

        let mmap_size = kvm.vcpu_mmap_size()?;

        // mmap the KVM run structure
        let kvm_run = unsafe {
            libc::mmap(
                std::ptr::null_mut(),
                mmap_size,
                libc::PROT_READ | libc::PROT_WRITE,
                libc::MAP_SHARED,
                fd,
                0,
            )
        };

        if kvm_run == libc::MAP_FAILED {
            unsafe { libc::close(fd) };
            return Err(VmmError::VCpuCreationFailed(
                "Failed to mmap KVM run structure".to_string(),
            ));
        }

        tracing::debug!("Created vCPU {}", id);

        Ok(Self {
            fd,
            id,
            kvm,
            state: VCpuState::Created,
            kvm_run: Some(kvm_run.cast()),
        })
    }

    /// Run the vCPU (blocking)
    pub fn run(&mut self) -> VmmResult<VCpuState> {
        self.state = VCpuState::Running;

        let ret = unsafe { libc::ioctl(self.fd, KVM_RUN, 0) };

        if ret < 0 {
            self.state = VCpuState::Error;
            return Err(VmmError::VCpuRunFailed(format!(
                "KVM_RUN failed: {}",
                std::io::Error::last_os_error()
            )));
        }

        // Check exit reason
        let exit_reason = unsafe {
            let run = self.kvm_run.expect("kvm_run should be mapped - mmap succeeded in new()");
            (*run).exit_reason
        };

        self.state = match exit_reason {
            KVM_EXIT_IO => VCpuState::IoExit,
            KVM_EXIT_MMIO => VCpuState::MmioExit,
            KVM_EXIT_HLT => VCpuState::Halted,
            KVM_EXIT_SHUTDOWN => VCpuState::Shutdown,
            KVM_EXIT_INTERNAL_ERROR => VCpuState::Error,
            _ => {
                tracing::warn!("Unknown KVM exit reason: {}", exit_reason);
                VCpuState::Error
            }
        };

        Ok(self.state)
    }

    /// Get vCPU ID
    pub fn id(&self) -> u32 {
        self.id
    }

    /// Get current state
    pub fn state(&self) -> VCpuState {
        self.state
    }

    /// Get the raw file descriptor
    pub fn as_raw_fd(&self) -> RawFd {
        self.fd
    }

    /// Request immediate exit from KVM_RUN
    pub fn request_exit(&mut self) {
        if let Some(run) = self.kvm_run {
            unsafe {
                (*run).immediate_exit = 1;
            }
        }
    }
}

impl Drop for VCpu {
    fn drop(&mut self) {
        if let Some(kvm_run) = self.kvm_run.take() {
            if let Ok(mmap_size) = self.kvm.vcpu_mmap_size() {
                unsafe {
                    libc::munmap(kvm_run.cast(), mmap_size);
                }
            }
        }
        unsafe {
            libc::close(self.fd);
        }
        tracing::debug!("vCPU {} destroyed", self.id);
    }
}

// Safety: VCpu can be sent between threads but not shared
// The mmap'd memory is tied to this vCPU and the fd
unsafe impl Send for VCpu {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_vcpu_state() {
        assert_eq!(VCpuState::Created, VCpuState::Created);
        assert_ne!(VCpuState::Created, VCpuState::Running);
    }
}
