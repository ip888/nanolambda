//! Guest memory management
//!
//! Provides safe abstractions over KVM's user memory regions.
//! Memory is allocated in the host address space and mapped into the guest.

use crate::error::{VmmError, VmmResult};
use std::collections::BTreeMap;
use std::os::unix::io::RawFd;
use std::ptr::NonNull;
use std::sync::atomic::{AtomicU32, Ordering};

/// Represents a region of guest memory
#[derive(Debug)]
pub struct MemoryRegion {
    /// Slot identifier
    pub slot: u32,
    /// Guest physical address start
    pub guest_phys_addr: u64,
    /// Size in bytes
    pub size: u64,
    /// Host virtual address
    host_addr: NonNull<u8>,
    /// Whether this region is read-only
    pub readonly: bool,
}

// Safety: MemoryRegion can be sent between threads
// The underlying memory is managed by mmap and is thread-safe
unsafe impl Send for MemoryRegion {}
unsafe impl Sync for MemoryRegion {}

impl MemoryRegion {
    /// Create a new memory region with mmap'd memory
    pub fn new(slot: u32, guest_phys_addr: u64, size: u64, readonly: bool) -> VmmResult<Self> {
        // Align size to page boundary
        let page_size = 4096u64;
        let aligned_size = (size + page_size - 1) & !(page_size - 1);

        // mmap anonymous memory
        let prot = if readonly {
            libc::PROT_READ
        } else {
            libc::PROT_READ | libc::PROT_WRITE
        };

        let host_addr = unsafe {
            libc::mmap(
                std::ptr::null_mut(),
                aligned_size as usize,
                prot,
                libc::MAP_PRIVATE | libc::MAP_ANONYMOUS | libc::MAP_NORESERVE,
                -1,
                0,
            )
        };

        if host_addr == libc::MAP_FAILED {
            return Err(VmmError::MemoryAllocationFailed(format!(
                "mmap failed for {} bytes",
                aligned_size
            )));
        }

        let host_addr = NonNull::new(host_addr.cast::<u8>())
            .ok_or_else(|| VmmError::MemoryAllocationFailed("mmap returned null".to_string()))?;

        Ok(Self {
            slot,
            guest_phys_addr,
            size: aligned_size,
            host_addr,
            readonly,
        })
    }

    /// Get the host virtual address
    pub fn host_addr(&self) -> *mut u8 {
        self.host_addr.as_ptr()
    }

    /// Write data to the region at an offset
    pub fn write(&self, offset: u64, data: &[u8]) -> VmmResult<()> {
        if self.readonly {
            return Err(VmmError::InvalidMemoryRegion(
                "Cannot write to read-only region".to_string(),
            ));
        }
        if offset + data.len() as u64 > self.size {
            return Err(VmmError::InvalidMemoryRegion(
                "Write exceeds region bounds".to_string(),
            ));
        }

        unsafe {
            std::ptr::copy_nonoverlapping(
                data.as_ptr(),
                self.host_addr.as_ptr().add(offset as usize),
                data.len(),
            );
        }
        Ok(())
    }

    /// Read data from the region at an offset
    pub fn read(&self, offset: u64, size: usize) -> VmmResult<Vec<u8>> {
        if offset + size as u64 > self.size {
            return Err(VmmError::InvalidMemoryRegion(
                "Read exceeds region bounds".to_string(),
            ));
        }

        let mut data = vec![0u8; size];
        unsafe {
            std::ptr::copy_nonoverlapping(
                self.host_addr.as_ptr().add(offset as usize),
                data.as_mut_ptr(),
                size,
            );
        }
        Ok(data)
    }

    /// Get the guest physical address of this region
    pub fn guest_addr(&self) -> u64 {
        self.guest_phys_addr
    }

    /// Get the size of this region in bytes
    pub fn region_size(&self) -> u64 {
        self.size
    }

    /// Check if this region is read-only
    pub fn is_readonly(&self) -> bool {
        self.readonly
    }

    /// Get the region contents as a slice
    ///
    /// # Safety
    /// The caller must ensure no concurrent writes occur during the slice lifetime.
    pub fn as_slice(&self) -> &[u8] {
        unsafe { std::slice::from_raw_parts(self.host_addr.as_ptr(), self.size as usize) }
    }
}

impl Drop for MemoryRegion {
    fn drop(&mut self) {
        unsafe {
            libc::munmap(self.host_addr.as_ptr().cast(), self.size as usize);
        }
    }
}

/// Memory slot identifier
pub type MemorySlot = u32;

/// Guest memory manager
pub struct GuestMemory {
    /// VM file descriptor
    vm_fd: RawFd,
    /// Memory regions by slot
    regions: BTreeMap<MemorySlot, MemoryRegion>,
    /// Next available slot
    next_slot: AtomicU32,
    /// Total memory size
    total_size: u64,
}

/// KVM user memory region structure (from linux/kvm.h)
#[repr(C)]
struct KvmUserspaceMemoryRegion {
    slot: u32,
    flags: u32,
    guest_phys_addr: u64,
    memory_size: u64,
    userspace_addr: u64,
}

const KVM_SET_USER_MEMORY_REGION: u64 = 0x4020_AE46;
const KVM_MEM_READONLY: u32 = 1 << 1;

impl GuestMemory {
    /// Create a new guest memory manager
    pub fn new(vm_fd: RawFd) -> Self {
        Self {
            vm_fd,
            regions: BTreeMap::new(),
            next_slot: AtomicU32::new(0),
            total_size: 0,
        }
    }

    /// Add a memory region
    pub fn add_region(
        &mut self,
        guest_phys_addr: u64,
        size: u64,
        readonly: bool,
    ) -> VmmResult<MemorySlot> {
        // Check for overlaps
        for region in self.regions.values() {
            let region_end = region.guest_phys_addr + region.size;
            let new_end = guest_phys_addr + size;

            if guest_phys_addr < region_end && new_end > region.guest_phys_addr {
                return Err(VmmError::MemoryRegionOverlap);
            }
        }

        let slot = self.next_slot.fetch_add(1, Ordering::SeqCst);
        let region = MemoryRegion::new(slot, guest_phys_addr, size, readonly)?;

        // Register with KVM
        let kvm_region = KvmUserspaceMemoryRegion {
            slot,
            flags: if readonly { KVM_MEM_READONLY } else { 0 },
            guest_phys_addr,
            memory_size: region.size,
            userspace_addr: region.host_addr() as u64,
        };

        let ret = unsafe {
            libc::ioctl(
                self.vm_fd,
                KVM_SET_USER_MEMORY_REGION,
                &kvm_region as *const _,
            )
        };

        if ret < 0 {
            return Err(VmmError::KvmIoctlFailed("KVM_SET_USER_MEMORY_REGION"));
        }

        self.total_size += region.size;
        self.regions.insert(slot, region);

        tracing::debug!(
            "Added memory region: slot={}, guest_addr=0x{:x}, size={}MB",
            slot,
            guest_phys_addr,
            size / (1024 * 1024)
        );

        Ok(slot)
    }

    /// Remove a memory region
    pub fn remove_region(&mut self, slot: MemorySlot) -> VmmResult<()> {
        let region = self
            .regions
            .get(&slot)
            .ok_or_else(|| VmmError::InvalidMemoryRegion(format!("Slot {} not found", slot)))?;

        // Unregister from KVM by setting size to 0
        let kvm_region = KvmUserspaceMemoryRegion {
            slot,
            flags: 0,
            guest_phys_addr: region.guest_phys_addr,
            memory_size: 0,
            userspace_addr: 0,
        };

        let ret = unsafe {
            libc::ioctl(
                self.vm_fd,
                KVM_SET_USER_MEMORY_REGION,
                &kvm_region as *const _,
            )
        };

        if ret < 0 {
            return Err(VmmError::KvmIoctlFailed(
                "KVM_SET_USER_MEMORY_REGION (remove)",
            ));
        }

        if let Some(region) = self.regions.remove(&slot) {
            self.total_size -= region.size;
        }

        Ok(())
    }

    /// Get a reference to a memory region
    pub fn region(&self, slot: MemorySlot) -> Option<&MemoryRegion> {
        self.regions.get(&slot)
    }

    /// Get total memory size
    pub fn total_size(&self) -> u64 {
        self.total_size
    }

    /// Write to guest memory by guest physical address
    pub fn write_at(&self, guest_addr: u64, data: &[u8]) -> VmmResult<()> {
        for region in self.regions.values() {
            let region_end = region.guest_phys_addr + region.size;
            if guest_addr >= region.guest_phys_addr && guest_addr < region_end {
                let offset = guest_addr - region.guest_phys_addr;
                return region.write(offset, data);
            }
        }
        Err(VmmError::InvalidMemoryRegion(format!(
            "Guest address 0x{:x} not mapped",
            guest_addr
        )))
    }

    /// Read from guest memory by guest physical address
    pub fn read_at(&self, guest_addr: u64, size: usize) -> VmmResult<Vec<u8>> {
        for region in self.regions.values() {
            let region_end = region.guest_phys_addr + region.size;
            if guest_addr >= region.guest_phys_addr && guest_addr < region_end {
                let offset = guest_addr - region.guest_phys_addr;
                return region.read(offset, size);
            }
        }
        Err(VmmError::InvalidMemoryRegion(format!(
            "Guest address 0x{:x} not mapped",
            guest_addr
        )))
    }

    /// Get total memory size (alias for total_size)
    ///
    /// Returns the sum of all memory region sizes in bytes.
    pub fn size(&self) -> u64 {
        self.total_size
    }

    /// Get iterator over all memory regions
    ///
    /// Returns references to all registered memory regions.
    /// This is useful for snapshotting and memory inspection.
    pub fn regions(&self) -> impl Iterator<Item = &MemoryRegion> {
        self.regions.values()
    }

    /// Get number of memory regions
    pub fn region_count(&self) -> usize {
        self.regions.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_memory_region_write_read() {
        let region = MemoryRegion::new(0, 0, 4096, false).expect("Should allocate");
        let data = b"Hello, Guest!";
        region.write(0, data).expect("Should write");
        let read_back = region.read(0, data.len()).expect("Should read");
        assert_eq!(&read_back, data);
    }

    #[test]
    fn test_memory_region_bounds() {
        let region = MemoryRegion::new(0, 0, 4096, false).expect("Should allocate");
        let result = region.write(4090, b"Too long!!");
        assert!(result.is_err());
    }

    #[test]
    fn test_readonly_region() {
        let region = MemoryRegion::new(0, 0, 4096, true).expect("Should allocate");
        let result = region.write(0, b"test");
        assert!(result.is_err());
    }
}
