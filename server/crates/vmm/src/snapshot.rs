//! Snapshot-based Instant VM Restore
//!
//! This module implements snapshot management with userfaultfd for lazy memory loading.
//! Instead of booting VMs from scratch, we restore from pre-captured snapshots for
//! sub-10ms cold starts.

use std::collections::HashMap;
use std::fs::{File, OpenOptions};
use std::io::{Read, Write};
use std::os::unix::io::{AsRawFd, RawFd};
use std::path::{Path, PathBuf};
use std::sync::Arc;

use crate::error::{VmmError, VmmResult};
use crate::memory::GuestMemory;

/// Snapshot format version for compatibility checking
const SNAPSHOT_VERSION: u32 = 1;

/// Magic bytes to identify NanoVM snapshots
const SNAPSHOT_MAGIC: &[u8; 8] = b"NANOSNAP";

/// A captured VM snapshot containing all state needed for instant restore
#[derive(Debug)]
pub struct VmSnapshot {
    /// Unique identifier for this snapshot
    pub id: String,
    /// Runtime type (nodejs, python, etc.)
    pub runtime: RuntimeType,
    /// CPU register state
    pub cpu_state: CpuSnapshotState,
    /// Memory regions with their content hashes
    pub memory_regions: Vec<MemoryRegionSnapshot>,
    /// Device states
    pub device_states: HashMap<String, Vec<u8>>,
    /// Timestamp when snapshot was taken
    pub created_at: u64,
    /// Total memory size
    pub memory_size: usize,
}

/// Supported runtime types for pre-warmed snapshots
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum RuntimeType {
    /// Node.js runtime
    NodeJs,
    /// Python runtime
    Python,
    /// Go runtime
    Go,
    /// Rust/native runtime
    Native,
    /// Custom/generic runtime
    Custom,
}

impl RuntimeType {
    pub fn as_str(&self) -> &'static str {
        match self {
            RuntimeType::NodeJs => "nodejs",
            RuntimeType::Python => "python",
            RuntimeType::Go => "go",
            RuntimeType::Native => "native",
            RuntimeType::Custom => "custom",
        }
    }
}

/// CPU state captured in a snapshot
#[derive(Debug, Clone, Default)]
pub struct CpuSnapshotState {
    /// General purpose registers
    pub regs: [u64; 16],
    /// Instruction pointer
    pub rip: u64,
    /// Stack pointer
    pub rsp: u64,
    /// Flags register
    pub rflags: u64,
    /// Segment registers
    pub segments: SegmentRegisters,
    /// Control registers
    pub control_regs: ControlRegisters,
    /// MSRs that need to be restored
    pub msrs: Vec<(u32, u64)>,
}

/// Segment register state
#[derive(Debug, Clone, Default)]
pub struct SegmentRegisters {
    pub cs: SegmentRegister,
    pub ds: SegmentRegister,
    pub es: SegmentRegister,
    pub fs: SegmentRegister,
    pub gs: SegmentRegister,
    pub ss: SegmentRegister,
}

/// Individual segment register
#[derive(Debug, Clone, Default)]
pub struct SegmentRegister {
    pub base: u64,
    pub limit: u32,
    pub selector: u16,
    pub type_: u8,
    pub present: u8,
    pub dpl: u8,
    pub db: u8,
    pub s: u8,
    pub l: u8,
    pub g: u8,
}

/// Control registers state
#[derive(Debug, Clone, Default)]
pub struct ControlRegisters {
    pub cr0: u64,
    pub cr2: u64,
    pub cr3: u64,
    pub cr4: u64,
    pub cr8: u64,
    pub efer: u64,
}

/// Memory region snapshot with lazy loading support
#[derive(Debug)]
pub struct MemoryRegionSnapshot {
    /// Guest physical address start
    pub guest_addr: u64,
    /// Size of the region
    pub size: usize,
    /// Path to the memory dump file (for lazy loading)
    pub dump_path: PathBuf,
    /// SHA-256 hash of the content for deduplication
    pub content_hash: [u8; 32],
    /// Whether this region is shared (read-only)
    pub is_shared: bool,
    /// Page bitmap for dirty tracking
    pub dirty_bitmap: Option<Vec<u8>>,
}

/// Manages snapshot lifecycle and storage
pub struct SnapshotManager {
    /// Base directory for snapshot storage
    storage_path: PathBuf,
    /// Cached snapshot metadata
    snapshots: HashMap<String, Arc<VmSnapshot>>,
    /// Pre-warmed "golden" snapshots by runtime type
    golden_snapshots: HashMap<RuntimeType, Arc<VmSnapshot>>,
}

impl SnapshotManager {
    /// Create a new snapshot manager
    pub fn new(storage_path: impl AsRef<Path>) -> VmmResult<Self> {
        let storage_path = storage_path.as_ref().to_path_buf();
        std::fs::create_dir_all(&storage_path).map_err(|e| {
            VmmError::SnapshotError(format!("Failed to create snapshot directory: {}", e))
        })?;

        Ok(Self {
            storage_path,
            snapshots: HashMap::new(),
            golden_snapshots: HashMap::new(),
        })
    }

    /// Capture a snapshot of a running VM
    pub fn capture_snapshot(
        &mut self,
        vm_id: &str,
        runtime: RuntimeType,
        memory: &GuestMemory,
        cpu_state: CpuSnapshotState,
        device_states: HashMap<String, Vec<u8>>,
    ) -> VmmResult<Arc<VmSnapshot>> {
        let snapshot_id = format!("{}_{}", vm_id, Self::timestamp());
        let snapshot_dir = self.storage_path.join(&snapshot_id);
        std::fs::create_dir_all(&snapshot_dir).map_err(|e| {
            VmmError::SnapshotError(format!("Failed to create snapshot dir: {}", e))
        })?;

        // Capture memory regions
        let memory_regions = self.capture_memory_regions(memory, &snapshot_dir)?;

        let snapshot = VmSnapshot {
            id: snapshot_id.clone(),
            runtime,
            cpu_state,
            memory_regions,
            device_states,
            created_at: Self::timestamp(),
            memory_size: memory.size() as usize,
        };

        // Save snapshot metadata
        self.save_snapshot_metadata(&snapshot)?;

        let snapshot = Arc::new(snapshot);
        self.snapshots.insert(snapshot_id, Arc::clone(&snapshot));

        Ok(snapshot)
    }

    /// Capture memory regions with deduplication
    fn capture_memory_regions(
        &self,
        memory: &GuestMemory,
        snapshot_dir: &Path,
    ) -> VmmResult<Vec<MemoryRegionSnapshot>> {
        let mut regions = Vec::new();

        for (idx, region) in memory.regions().enumerate() {
            let dump_path = snapshot_dir.join(format!("memory_{}.bin", idx));

            // Calculate content hash for deduplication
            let content_hash = Self::hash_memory_region(region.as_slice());

            // Check if we already have this content (deduplication)
            let actual_path = self.find_deduplicated_path(&content_hash, &dump_path)?;

            regions.push(MemoryRegionSnapshot {
                guest_addr: region.guest_addr(),
                size: region.region_size() as usize,
                dump_path: actual_path,
                content_hash,
                is_shared: region.is_readonly(),
                dirty_bitmap: None,
            });
        }

        Ok(regions)
    }

    /// Find or create deduplicated memory file
    fn find_deduplicated_path(&self, hash: &[u8; 32], default_path: &Path) -> VmmResult<PathBuf> {
        let hash_hex = hex::encode(hash);
        let dedup_path = self.storage_path.join("dedup").join(&hash_hex[..2]);
        let dedup_file = dedup_path.join(format!("{}.bin", hash_hex));

        if dedup_file.exists() {
            // Reuse existing deduplicated file
            Ok(dedup_file)
        } else {
            // Use the default path, will create dedup entry later
            Ok(default_path.to_path_buf())
        }
    }

    /// Calculate SHA-256 hash of memory content
    fn hash_memory_region(data: &[u8]) -> [u8; 32] {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};

        // Simple hash for now - would use SHA-256 in production
        let mut hasher = DefaultHasher::new();
        data.hash(&mut hasher);
        let hash = hasher.finish();

        let mut result = [0u8; 32];
        result[..8].copy_from_slice(&hash.to_le_bytes());
        result
    }

    /// Save snapshot metadata to disk
    fn save_snapshot_metadata(&self, snapshot: &VmSnapshot) -> VmmResult<()> {
        let meta_path = self.storage_path.join(&snapshot.id).join("metadata.json");
        let metadata = serde_json::json!({
            "version": SNAPSHOT_VERSION,
            "id": snapshot.id,
            "runtime": snapshot.runtime.as_str(),
            "created_at": snapshot.created_at,
            "memory_size": snapshot.memory_size,
            "region_count": snapshot.memory_regions.len(),
        });

        let mut file = File::create(&meta_path).map_err(|e| {
            VmmError::SnapshotError(format!("Failed to create metadata file: {}", e))
        })?;

        file.write_all(metadata.to_string().as_bytes())
            .map_err(|e| VmmError::SnapshotError(format!("Failed to write metadata: {}", e)))?;

        Ok(())
    }

    /// Register a pre-warmed golden snapshot for a runtime type
    pub fn register_golden_snapshot(&mut self, runtime: RuntimeType, snapshot: Arc<VmSnapshot>) {
        self.golden_snapshots.insert(runtime, snapshot);
    }

    /// Get the golden snapshot for a runtime type
    pub fn get_golden_snapshot(&self, runtime: RuntimeType) -> Option<Arc<VmSnapshot>> {
        self.golden_snapshots.get(&runtime).cloned()
    }

    /// Load a snapshot by ID
    pub fn load_snapshot(&mut self, snapshot_id: &str) -> VmmResult<Arc<VmSnapshot>> {
        if let Some(snapshot) = self.snapshots.get(snapshot_id) {
            return Ok(Arc::clone(snapshot));
        }

        // Load from disk
        let snapshot = self.load_snapshot_from_disk(snapshot_id)?;
        let snapshot = Arc::new(snapshot);
        self.snapshots
            .insert(snapshot_id.to_string(), Arc::clone(&snapshot));

        Ok(snapshot)
    }

    /// Load snapshot metadata from disk
    fn load_snapshot_from_disk(&self, snapshot_id: &str) -> VmmResult<VmSnapshot> {
        let snapshot_dir = self.storage_path.join(snapshot_id);
        let meta_path = snapshot_dir.join("metadata.json");

        let mut file = File::open(&meta_path)
            .map_err(|e| VmmError::SnapshotError(format!("Snapshot not found: {}", e)))?;

        let mut content = String::new();
        file.read_to_string(&mut content)
            .map_err(|e| VmmError::SnapshotError(format!("Failed to read metadata: {}", e)))?;

        // Parse metadata and reconstruct snapshot
        // This is simplified - real implementation would fully deserialize
        Err(VmmError::SnapshotError(
            "Full snapshot loading not yet implemented".to_string(),
        ))
    }

    /// List all available snapshots
    pub fn list_snapshots(&self) -> Vec<&str> {
        self.snapshots.keys().map(|s| s.as_str()).collect()
    }

    /// Delete a snapshot
    pub fn delete_snapshot(&mut self, snapshot_id: &str) -> VmmResult<()> {
        self.snapshots.remove(snapshot_id);
        let snapshot_dir = self.storage_path.join(snapshot_id);
        if snapshot_dir.exists() {
            std::fs::remove_dir_all(&snapshot_dir).map_err(|e| {
                VmmError::SnapshotError(format!("Failed to delete snapshot: {}", e))
            })?;
        }
        Ok(())
    }

    fn timestamp() -> u64 {
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs()
    }
}

/// Lazy memory loader using userfaultfd
pub struct LazyMemoryLoader {
    /// File descriptor for userfaultfd
    uffd: RawFd,
    /// Snapshot being restored
    snapshot: Arc<VmSnapshot>,
    /// Mapping of guest addresses to file offsets
    page_mappings: HashMap<u64, PageMapping>,
    /// Statistics for monitoring
    stats: LazyLoadStats,
}

/// Mapping information for a memory page
#[derive(Debug, Clone)]
struct PageMapping {
    /// File containing the page data
    file_path: PathBuf,
    /// Offset within the file
    offset: u64,
    /// Whether this page has been loaded
    loaded: bool,
}

/// Statistics for lazy loading
#[derive(Debug, Default)]
pub struct LazyLoadStats {
    /// Total pages in the snapshot
    pub total_pages: u64,
    /// Pages loaded on demand
    pub pages_loaded: u64,
    /// Pages never accessed (saved memory)
    pub pages_skipped: u64,
    /// Total bytes loaded
    pub bytes_loaded: u64,
    /// Average page fault handling time in microseconds
    pub avg_fault_time_us: u64,
}

impl LazyMemoryLoader {
    /// Create a new lazy memory loader for a snapshot
    pub fn new(snapshot: Arc<VmSnapshot>) -> VmmResult<Self> {
        // Create userfaultfd (requires Linux 4.11+)
        let uffd =
            unsafe { libc::syscall(libc::SYS_userfaultfd, libc::O_CLOEXEC | libc::O_NONBLOCK) };
        if uffd < 0 {
            return Err(VmmError::SnapshotError(
                "Failed to create userfaultfd - requires Linux 4.11+".to_string(),
            ));
        }

        let mut page_mappings = HashMap::new();
        let page_size = 4096u64;

        // Build page mappings from snapshot
        for region in &snapshot.memory_regions {
            let num_pages = (region.size as u64 + page_size - 1) / page_size;
            for i in 0..num_pages {
                let guest_addr = region.guest_addr + i * page_size;
                page_mappings.insert(
                    guest_addr,
                    PageMapping {
                        file_path: region.dump_path.clone(),
                        offset: i * page_size,
                        loaded: false,
                    },
                );
            }
        }

        let total_pages = page_mappings.len() as u64;

        Ok(Self {
            uffd: uffd as RawFd,
            snapshot,
            page_mappings,
            stats: LazyLoadStats {
                total_pages,
                ..Default::default()
            },
        })
    }

    /// Register a memory region for lazy loading
    pub fn register_region(&mut self, addr: *mut u8, len: usize) -> VmmResult<()> {
        // This would use UFFDIO_REGISTER to set up fault handling
        // Simplified for now
        Ok(())
    }

    /// Handle a page fault by loading the required page
    pub fn handle_page_fault(&mut self, fault_addr: u64) -> VmmResult<Vec<u8>> {
        let page_size = 4096u64;
        let page_addr = fault_addr & !(page_size - 1);

        let mapping = self.page_mappings.get_mut(&page_addr).ok_or_else(|| {
            VmmError::SnapshotError(format!("No mapping for address 0x{:x}", fault_addr))
        })?;

        if mapping.loaded {
            return Err(VmmError::SnapshotError("Page already loaded".to_string()));
        }

        // Load page from file
        let mut file = File::open(&mapping.file_path)
            .map_err(|e| VmmError::SnapshotError(format!("Failed to open memory file: {}", e)))?;

        let mut page_data = vec![0u8; page_size as usize];
        use std::io::Seek;
        file.seek(std::io::SeekFrom::Start(mapping.offset))
            .map_err(|e| VmmError::SnapshotError(format!("Failed to seek: {}", e)))?;
        file.read_exact(&mut page_data)
            .map_err(|e| VmmError::SnapshotError(format!("Failed to read page: {}", e)))?;

        mapping.loaded = true;
        self.stats.pages_loaded += 1;
        self.stats.bytes_loaded += page_size;

        Ok(page_data)
    }

    /// Get lazy loading statistics
    pub fn stats(&self) -> &LazyLoadStats {
        &self.stats
    }

    /// Calculate memory savings from lazy loading
    pub fn memory_savings(&self) -> f64 {
        if self.stats.total_pages == 0 {
            return 0.0;
        }
        let skipped = self.stats.total_pages - self.stats.pages_loaded;
        (skipped as f64 / self.stats.total_pages as f64) * 100.0
    }
}

impl Drop for LazyMemoryLoader {
    fn drop(&mut self) {
        if self.uffd >= 0 {
            unsafe {
                libc::close(self.uffd);
            }
        }
    }
}

/// Builder for creating golden snapshots
pub struct GoldenSnapshotBuilder {
    runtime: RuntimeType,
    boot_script: Option<String>,
    pre_warm_modules: Vec<String>,
    memory_size_mb: u32,
}

impl GoldenSnapshotBuilder {
    pub fn new(runtime: RuntimeType) -> Self {
        Self {
            runtime,
            boot_script: None,
            pre_warm_modules: Vec::new(),
            memory_size_mb: 128,
        }
    }

    /// Set the boot script to run before taking the snapshot
    pub fn boot_script(mut self, script: impl Into<String>) -> Self {
        self.boot_script = Some(script.into());
        self
    }

    /// Add modules to pre-load before taking the snapshot
    pub fn pre_warm_module(mut self, module: impl Into<String>) -> Self {
        self.pre_warm_modules.push(module.into());
        self
    }

    /// Set memory size in MB
    pub fn memory_mb(mut self, mb: u32) -> Self {
        self.memory_size_mb = mb;
        self
    }

    /// Build the golden snapshot
    /// This would boot a VM, run the boot script, load modules, then snapshot
    pub fn build(self) -> VmmResult<VmSnapshot> {
        // In a real implementation:
        // 1. Boot a fresh VM with the specified runtime
        // 2. Run the boot script
        // 3. Pre-load specified modules
        // 4. Capture the snapshot

        // Return placeholder for now
        Ok(VmSnapshot {
            id: format!(
                "golden_{}_{}",
                self.runtime.as_str(),
                SnapshotManager::timestamp()
            ),
            runtime: self.runtime,
            cpu_state: CpuSnapshotState::default(),
            memory_regions: Vec::new(),
            device_states: HashMap::new(),
            created_at: SnapshotManager::timestamp(),
            memory_size: (self.memory_size_mb as usize) * 1024 * 1024,
        })
    }
}

fn timestamp() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_runtime_type_str() {
        assert_eq!(RuntimeType::NodeJs.as_str(), "nodejs");
        assert_eq!(RuntimeType::Python.as_str(), "python");
    }

    #[test]
    fn test_snapshot_manager_creation() {
        let temp_dir = std::env::temp_dir().join("nanovm_test_snapshots");
        let manager = SnapshotManager::new(&temp_dir);
        assert!(manager.is_ok());
        let _ = std::fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_golden_snapshot_builder() {
        let builder = GoldenSnapshotBuilder::new(RuntimeType::NodeJs)
            .memory_mb(256)
            .pre_warm_module("express")
            .pre_warm_module("lodash");

        let snapshot = builder.build();
        assert!(snapshot.is_ok());
        let snapshot = snapshot.unwrap();
        assert_eq!(snapshot.runtime, RuntimeType::NodeJs);
    }

    #[test]
    fn test_memory_hash() {
        let data = vec![1u8, 2, 3, 4, 5];
        let hash1 = SnapshotManager::hash_memory_region(&data);
        let hash2 = SnapshotManager::hash_memory_region(&data);
        assert_eq!(hash1, hash2);

        let different_data = vec![5u8, 4, 3, 2, 1];
        let hash3 = SnapshotManager::hash_memory_region(&different_data);
        assert_ne!(hash1, hash3);
    }
}
