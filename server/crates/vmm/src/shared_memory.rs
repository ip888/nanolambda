//! Shared Memory Optimization with KSM
//!
//! Implements memory sharing between VM instances for maximum density.
//! Uses Kernel Same-page Merging (KSM) and explicit shared regions for
//! common runtime code (Node.js, Python, etc.)

use std::collections::HashMap;
use std::fs::{File, OpenOptions};
use std::io::{Read, Write};
use std::os::unix::io::AsRawFd;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};
use std::sync::{Arc, RwLock};

use crate::error::{VmmError, VmmResult};
use crate::snapshot::RuntimeType;

/// Page size (4KB)
const PAGE_SIZE: usize = 4096;

/// Configuration for shared memory
#[derive(Debug, Clone)]
pub struct SharedMemoryConfig {
    /// Enable KSM for automatic page deduplication
    pub enable_ksm: bool,
    /// KSM pages to scan per cycle
    pub ksm_pages_to_scan: u32,
    /// KSM sleep between scans (milliseconds)
    pub ksm_sleep_ms: u32,
    /// Enable explicit runtime sharing
    pub enable_runtime_sharing: bool,
    /// Base path for shared memory files
    pub shared_path: PathBuf,
    /// Maximum shared memory size per runtime
    pub max_shared_per_runtime: usize,
}

impl Default for SharedMemoryConfig {
    fn default() -> Self {
        Self {
            enable_ksm: true,
            ksm_pages_to_scan: 100,
            ksm_sleep_ms: 20,
            enable_runtime_sharing: true,
            shared_path: PathBuf::from("/dev/shm/nanovm"),
            max_shared_per_runtime: 256 * 1024 * 1024, // 256 MB
        }
    }
}

/// A shared memory region that can be mapped into multiple VMs
#[derive(Debug)]
pub struct SharedRegion {
    /// Unique identifier
    pub id: String,
    /// Runtime type this region is for
    pub runtime: RuntimeType,
    /// Memory-mapped region
    mmap: MappedRegion,
    /// Reference count
    ref_count: AtomicUsize,
    /// Statistics
    stats: SharedRegionStats,
}

/// Statistics for a shared region
#[derive(Debug, Default)]
pub struct SharedRegionStats {
    /// Number of VMs using this region
    pub users: AtomicUsize,
    /// Total bytes shared
    pub bytes_shared: AtomicU64,
    /// Memory saved (vs duplicating)
    pub memory_saved: AtomicU64,
}

/// Memory-mapped region
#[derive(Debug)]
struct MappedRegion {
    /// Pointer to mapped memory
    ptr: *mut u8,
    /// Size of the mapping
    size: usize,
    /// File descriptor (for file-backed mappings)
    fd: Option<i32>,
    /// Path to backing file
    path: Option<PathBuf>,
}

// Safety: MappedRegion is thread-safe because we only allow read access
// to the shared memory and coordinate writes through proper synchronization
unsafe impl Send for MappedRegion {}
unsafe impl Sync for MappedRegion {}

impl MappedRegion {
    /// Create a new anonymous mapping
    pub fn anonymous(size: usize) -> VmmResult<Self> {
        let ptr = unsafe {
            libc::mmap(
                std::ptr::null_mut(),
                size,
                libc::PROT_READ | libc::PROT_WRITE,
                libc::MAP_PRIVATE | libc::MAP_ANONYMOUS,
                -1,
                0,
            )
        };

        if ptr == libc::MAP_FAILED {
            return Err(VmmError::MemoryError("mmap failed".to_string()));
        }

        Ok(Self {
            ptr: ptr as *mut u8,
            size,
            fd: None,
            path: None,
        })
    }

    /// Create a file-backed mapping
    pub fn file_backed(path: &Path, size: usize) -> VmmResult<Self> {
        // Create or open the file
        let file = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .open(path)
            .map_err(|e| VmmError::MemoryError(format!("Failed to open file: {}", e)))?;

        // Set file size
        file.set_len(size as u64)
            .map_err(|e| VmmError::MemoryError(format!("Failed to set file size: {}", e)))?;

        let fd = file.as_raw_fd();

        let ptr = unsafe {
            libc::mmap(
                std::ptr::null_mut(),
                size,
                libc::PROT_READ | libc::PROT_WRITE,
                libc::MAP_SHARED,
                fd,
                0,
            )
        };

        if ptr == libc::MAP_FAILED {
            return Err(VmmError::MemoryError("mmap failed".to_string()));
        }

        // Keep file open
        std::mem::forget(file);

        Ok(Self {
            ptr: ptr as *mut u8,
            size,
            fd: Some(fd),
            path: Some(path.to_path_buf()),
        })
    }

    /// Get a read-only slice of the mapping
    pub fn as_slice(&self) -> &[u8] {
        unsafe { std::slice::from_raw_parts(self.ptr, self.size) }
    }

    /// Get a mutable slice of the mapping
    pub fn as_mut_slice(&mut self) -> &mut [u8] {
        unsafe { std::slice::from_raw_parts_mut(self.ptr, self.size) }
    }

    /// Get the raw pointer
    pub fn as_ptr(&self) -> *const u8 {
        self.ptr
    }

    /// Get size
    pub fn size(&self) -> usize {
        self.size
    }

    /// Mark region for KSM scanning
    pub fn enable_ksm(&self) -> VmmResult<()> {
        let result = unsafe {
            libc::madvise(
                self.ptr as *mut libc::c_void,
                self.size,
                libc::MADV_MERGEABLE,
            )
        };

        if result != 0 {
            return Err(VmmError::MemoryError(
                "madvise MERGEABLE failed".to_string(),
            ));
        }

        Ok(())
    }

    /// Disable KSM for this region
    pub fn disable_ksm(&self) -> VmmResult<()> {
        let result = unsafe {
            libc::madvise(
                self.ptr as *mut libc::c_void,
                self.size,
                libc::MADV_UNMERGEABLE,
            )
        };

        if result != 0 {
            // Not an error if unmergeable not supported
        }

        Ok(())
    }
}

impl Drop for MappedRegion {
    fn drop(&mut self) {
        unsafe {
            libc::munmap(self.ptr as *mut libc::c_void, self.size);
            if let Some(fd) = self.fd {
                libc::close(fd);
            }
        }
    }
}

impl SharedRegion {
    /// Create a new shared region
    pub fn new(
        id: impl Into<String>,
        runtime: RuntimeType,
        size: usize,
        path: Option<&Path>,
    ) -> VmmResult<Self> {
        let mmap = if let Some(path) = path {
            MappedRegion::file_backed(path, size)?
        } else {
            MappedRegion::anonymous(size)?
        };

        Ok(Self {
            id: id.into(),
            runtime,
            mmap,
            ref_count: AtomicUsize::new(0),
            stats: SharedRegionStats::default(),
        })
    }

    /// Acquire a reference to this shared region
    pub fn acquire(&self) -> SharedRegionRef {
        self.ref_count.fetch_add(1, Ordering::SeqCst);
        self.stats.users.fetch_add(1, Ordering::Relaxed);

        SharedRegionRef {
            ptr: self.mmap.as_ptr(),
            size: self.mmap.size(),
        }
    }

    /// Release a reference
    pub fn release(&self) {
        self.ref_count.fetch_sub(1, Ordering::SeqCst);
        self.stats.users.fetch_sub(1, Ordering::Relaxed);
    }

    /// Get current reference count
    pub fn ref_count(&self) -> usize {
        self.ref_count.load(Ordering::SeqCst)
    }

    /// Load data into the shared region
    pub fn load_data(&mut self, data: &[u8]) -> VmmResult<()> {
        if data.len() > self.mmap.size() {
            return Err(VmmError::MemoryError(
                "Data too large for region".to_string(),
            ));
        }

        self.mmap.as_mut_slice()[..data.len()].copy_from_slice(data);
        self.stats
            .bytes_shared
            .store(data.len() as u64, Ordering::Relaxed);

        Ok(())
    }

    /// Enable KSM for this region
    pub fn enable_ksm(&self) -> VmmResult<()> {
        self.mmap.enable_ksm()
    }

    /// Get statistics
    pub fn stats(&self) -> &SharedRegionStats {
        &self.stats
    }

    /// Calculate memory saved by sharing
    pub fn memory_saved(&self) -> u64 {
        let users = self.stats.users.load(Ordering::Relaxed);
        if users <= 1 {
            return 0;
        }
        let bytes = self.stats.bytes_shared.load(Ordering::Relaxed);
        bytes * (users as u64 - 1)
    }
}

/// Reference to a shared region for mapping into a VM
#[derive(Debug, Clone, Copy)]
pub struct SharedRegionRef {
    pub ptr: *const u8,
    pub size: usize,
}

// Safety: SharedRegionRef is just a read-only pointer to shared memory
unsafe impl Send for SharedRegionRef {}
unsafe impl Sync for SharedRegionRef {}

/// Manager for shared memory regions
pub struct SharedMemoryManager {
    /// Configuration
    config: SharedMemoryConfig,
    /// Shared regions by ID
    regions: RwLock<HashMap<String, Arc<SharedRegion>>>,
    /// Regions by runtime type
    runtime_regions: RwLock<HashMap<RuntimeType, Vec<String>>>,
    /// KSM controller
    ksm: Option<KsmController>,
    /// Global statistics
    stats: SharedMemoryStats,
}

/// Global statistics for shared memory
#[derive(Debug, Default)]
pub struct SharedMemoryStats {
    /// Total shared regions
    pub total_regions: AtomicUsize,
    /// Total shared bytes
    pub total_shared_bytes: AtomicU64,
    /// Total memory saved
    pub total_memory_saved: AtomicU64,
    /// KSM pages merged
    pub ksm_pages_merged: AtomicU64,
}

impl SharedMemoryManager {
    /// Create a new shared memory manager
    pub fn new(config: SharedMemoryConfig) -> VmmResult<Self> {
        // Create shared memory directory
        if config.enable_runtime_sharing {
            std::fs::create_dir_all(&config.shared_path).map_err(|e| {
                VmmError::MemoryError(format!("Failed to create shared path: {}", e))
            })?;
        }

        let ksm = if config.enable_ksm {
            KsmController::new(&config).ok()
        } else {
            None
        };

        Ok(Self {
            config,
            regions: RwLock::new(HashMap::new()),
            runtime_regions: RwLock::new(HashMap::new()),
            ksm,
            stats: SharedMemoryStats::default(),
        })
    }

    /// Create a shared region for a runtime
    pub fn create_runtime_region(
        &self,
        runtime: RuntimeType,
        data: &[u8],
    ) -> VmmResult<Arc<SharedRegion>> {
        let id = format!(
            "runtime_{}_{}",
            runtime.as_str(),
            self.stats.total_regions.load(Ordering::Relaxed)
        );

        let path = if self.config.enable_runtime_sharing {
            Some(self.config.shared_path.join(&id))
        } else {
            None
        };

        let mut region = SharedRegion::new(&id, runtime, data.len(), path.as_deref())?;
        region.load_data(data)?;

        if self.config.enable_ksm {
            region.enable_ksm()?;
        }

        let region = Arc::new(region);

        // Store region
        self.regions
            .write()
            .unwrap()
            .insert(id.clone(), Arc::clone(&region));
        self.runtime_regions
            .write()
            .unwrap()
            .entry(runtime)
            .or_default()
            .push(id);

        self.stats.total_regions.fetch_add(1, Ordering::Relaxed);
        self.stats
            .total_shared_bytes
            .fetch_add(data.len() as u64, Ordering::Relaxed);

        Ok(region)
    }

    /// Get shared regions for a runtime type
    pub fn get_runtime_regions(&self, runtime: RuntimeType) -> Vec<Arc<SharedRegion>> {
        let runtime_regions = self.runtime_regions.read().unwrap();
        let regions = self.regions.read().unwrap();

        runtime_regions
            .get(&runtime)
            .map(|ids| {
                ids.iter()
                    .filter_map(|id| regions.get(id).cloned())
                    .collect()
            })
            .unwrap_or_default()
    }

    /// Get a specific shared region
    pub fn get_region(&self, id: &str) -> Option<Arc<SharedRegion>> {
        self.regions.read().unwrap().get(id).cloned()
    }

    /// Calculate total memory saved
    pub fn total_memory_saved(&self) -> u64 {
        let regions = self.regions.read().unwrap();
        regions.values().map(|r| r.memory_saved()).sum()
    }

    /// Get global statistics
    pub fn stats(&self) -> &SharedMemoryStats {
        &self.stats
    }

    /// Run maintenance (KSM scan, cleanup unused regions)
    pub fn maintenance(&self) -> MaintenanceResult {
        let mut result = MaintenanceResult::default();

        // Trigger KSM scan
        if let Some(ref ksm) = self.ksm {
            result.pages_merged = ksm.trigger_scan();
            self.stats
                .ksm_pages_merged
                .fetch_add(result.pages_merged, Ordering::Relaxed);
        }

        // Update memory saved stats
        let saved = self.total_memory_saved();
        self.stats
            .total_memory_saved
            .store(saved, Ordering::Relaxed);
        result.memory_saved = saved;

        // Cleanup unused regions
        let mut regions = self.regions.write().unwrap();
        let before = regions.len();
        regions.retain(|_, r| r.ref_count() > 0 || Arc::strong_count(r) > 1);
        result.regions_cleaned = before - regions.len();

        result
    }
}

/// Result of maintenance operation
#[derive(Debug, Default)]
pub struct MaintenanceResult {
    pub pages_merged: u64,
    pub memory_saved: u64,
    pub regions_cleaned: usize,
}

/// KSM (Kernel Same-page Merging) controller
struct KsmController {
    /// KSM sysfs path
    sysfs_path: PathBuf,
    /// Whether KSM is enabled on this system
    available: bool,
}

impl KsmController {
    fn new(config: &SharedMemoryConfig) -> VmmResult<Self> {
        let sysfs_path = PathBuf::from("/sys/kernel/mm/ksm");

        // Check if KSM is available
        if !sysfs_path.exists() {
            return Err(VmmError::FeatureNotSupported(
                "KSM not available".to_string(),
            ));
        }

        let controller = Self {
            sysfs_path,
            available: true,
        };

        // Configure KSM parameters
        controller.configure(config)?;

        Ok(controller)
    }

    fn configure(&self, config: &SharedMemoryConfig) -> VmmResult<()> {
        // Set pages to scan
        self.write_param("pages_to_scan", config.ksm_pages_to_scan)?;

        // Set sleep time
        self.write_param("sleep_millisecs", config.ksm_sleep_ms)?;

        // Enable KSM
        self.write_param("run", 1)?;

        Ok(())
    }

    fn write_param(&self, param: &str, value: u32) -> VmmResult<()> {
        let path = self.sysfs_path.join(param);
        std::fs::write(&path, value.to_string())
            .map_err(|e| VmmError::MemoryError(format!("Failed to set KSM {}: {}", param, e)))?;
        Ok(())
    }

    fn read_param(&self, param: &str) -> VmmResult<u64> {
        let path = self.sysfs_path.join(param);
        let content = std::fs::read_to_string(&path)
            .map_err(|e| VmmError::MemoryError(format!("Failed to read KSM {}: {}", param, e)))?;
        content
            .trim()
            .parse()
            .map_err(|e| VmmError::MemoryError(format!("Failed to parse KSM {}: {}", param, e)))
    }

    fn trigger_scan(&self) -> u64 {
        // Read pages_shared before and after to get delta
        let before = self.read_param("pages_shared").unwrap_or(0);

        // KSM runs automatically, but we can check current state
        let after = self.read_param("pages_shared").unwrap_or(0);

        after.saturating_sub(before)
    }

    /// Get KSM statistics
    pub fn stats(&self) -> KsmStats {
        KsmStats {
            pages_shared: self.read_param("pages_shared").unwrap_or(0),
            pages_sharing: self.read_param("pages_sharing").unwrap_or(0),
            pages_unshared: self.read_param("pages_unshared").unwrap_or(0),
            full_scans: self.read_param("full_scans").unwrap_or(0),
        }
    }
}

/// KSM statistics
#[derive(Debug, Default)]
pub struct KsmStats {
    /// Pages that are shared
    pub pages_shared: u64,
    /// Pages currently sharing
    pub pages_sharing: u64,
    /// Pages that couldn't be merged
    pub pages_unshared: u64,
    /// Number of full scans completed
    pub full_scans: u64,
}

impl KsmStats {
    /// Calculate memory saved by KSM (in bytes)
    pub fn memory_saved(&self) -> u64 {
        // pages_sharing - pages_shared = pages saved
        // (pages_sharing is total, pages_shared is unique)
        self.pages_sharing.saturating_sub(self.pages_shared) * PAGE_SIZE as u64
    }
}

/// Runtime memory loader for common runtimes
pub struct RuntimeMemoryLoader;

impl RuntimeMemoryLoader {
    /// Load Node.js runtime into shared memory
    pub fn load_nodejs(
        manager: &SharedMemoryManager,
        version: &str,
    ) -> VmmResult<Arc<SharedRegion>> {
        // In real implementation, this would:
        // 1. Load the Node.js binary and V8 heap snapshot
        // 2. Include commonly used modules (express, lodash, etc.)

        let runtime_data = Self::create_nodejs_template(version)?;
        manager.create_runtime_region(RuntimeType::NodeJs, &runtime_data)
    }

    /// Load Python runtime into shared memory
    pub fn load_python(
        manager: &SharedMemoryManager,
        version: &str,
    ) -> VmmResult<Arc<SharedRegion>> {
        let runtime_data = Self::create_python_template(version)?;
        manager.create_runtime_region(RuntimeType::Python, &runtime_data)
    }

    fn create_nodejs_template(_version: &str) -> VmmResult<Vec<u8>> {
        // Placeholder - real impl would include actual runtime
        Ok(vec![0u8; 30 * 1024 * 1024]) // 30 MB placeholder
    }

    fn create_python_template(_version: &str) -> VmmResult<Vec<u8>> {
        // Placeholder - real impl would include actual runtime
        Ok(vec![0u8; 15 * 1024 * 1024]) // 15 MB placeholder
    }
}

/// Memory density calculator
pub struct DensityCalculator;

impl DensityCalculator {
    /// Calculate how many VMs can fit in given memory with sharing
    pub fn calculate_density(
        total_memory: usize,
        vm_memory: usize,
        shared_size: usize,
        avg_private_pages: usize,
    ) -> DensityReport {
        let without_sharing = total_memory / vm_memory;

        let effective_per_vm = avg_private_pages * PAGE_SIZE;
        let with_sharing = if effective_per_vm > 0 {
            (total_memory - shared_size) / effective_per_vm
        } else {
            without_sharing
        };

        let improvement = if without_sharing > 0 {
            (with_sharing as f64 / without_sharing as f64 - 1.0) * 100.0
        } else {
            0.0
        };

        DensityReport {
            without_sharing,
            with_sharing,
            improvement_percent: improvement,
            memory_saved: (with_sharing - without_sharing) * vm_memory,
        }
    }
}

/// Density calculation report
#[derive(Debug)]
pub struct DensityReport {
    /// VMs possible without sharing
    pub without_sharing: usize,
    /// VMs possible with sharing
    pub with_sharing: usize,
    /// Improvement percentage
    pub improvement_percent: f64,
    /// Memory saved in bytes
    pub memory_saved: usize,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mapped_region_anonymous() {
        let region = MappedRegion::anonymous(4096);
        assert!(region.is_ok());

        let region = region.unwrap();
        assert_eq!(region.size(), 4096);
    }

    #[test]
    fn test_shared_region_creation() {
        let region = SharedRegion::new("test", RuntimeType::NodeJs, 4096, None);
        assert!(region.is_ok());

        let region = region.unwrap();
        assert_eq!(region.ref_count(), 0);
    }

    #[test]
    fn test_shared_region_acquire_release() {
        let region = SharedRegion::new("test", RuntimeType::NodeJs, 4096, None).unwrap();

        let _ref1 = region.acquire();
        assert_eq!(region.ref_count(), 1);

        let _ref2 = region.acquire();
        assert_eq!(region.ref_count(), 2);

        region.release();
        assert_eq!(region.ref_count(), 1);
    }

    #[test]
    fn test_memory_saved_calculation() {
        let mut region = SharedRegion::new("test", RuntimeType::NodeJs, 4096, None).unwrap();
        region.load_data(&[0u8; 4096]).unwrap();

        // With 1 user, no savings
        let _ref1 = region.acquire();
        assert_eq!(region.memory_saved(), 0);

        // With 2 users, save 4096 bytes
        let _ref2 = region.acquire();
        assert_eq!(region.memory_saved(), 4096);

        // With 3 users, save 8192 bytes
        let _ref3 = region.acquire();
        assert_eq!(region.memory_saved(), 8192);
    }

    #[test]
    fn test_density_calculator() {
        let report = DensityCalculator::calculate_density(
            1024 * 1024 * 1024, // 1 GB total
            128 * 1024 * 1024,  // 128 MB per VM
            30 * 1024 * 1024,   // 30 MB shared
            250,                // 250 private pages (1 MB)
        );

        assert!(report.with_sharing > report.without_sharing);
        assert!(report.improvement_percent > 0.0);
    }
}
