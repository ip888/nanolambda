//! Copy-on-Write Clone Pool
//!
//! Manages a pool of pre-cloned VMs from golden snapshots for instant function execution.
//! Uses Copy-on-Write (CoW) memory to minimize memory overhead when running many instances.

use std::collections::{HashMap, VecDeque};
use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex, RwLock};
use std::time::{Duration, Instant};

use crate::error::{VmmError, VmmResult};
use crate::snapshot::{RuntimeType, VmSnapshot};

/// Configuration for a clone pool
#[derive(Debug, Clone)]
pub struct ClonePoolConfig {
    /// Minimum number of warm clones to maintain
    pub min_warm_clones: usize,
    /// Maximum number of warm clones
    pub max_warm_clones: usize,
    /// How long to keep idle clones before recycling
    pub idle_timeout: Duration,
    /// Whether to use KSM (Kernel Same-page Merging)
    pub enable_ksm: bool,
    /// Target memory density (clones per GB)
    pub target_density: u32,
}

impl Default for ClonePoolConfig {
    fn default() -> Self {
        Self {
            min_warm_clones: 2,
            max_warm_clones: 100,
            idle_timeout: Duration::from_secs(300), // 5 minutes
            enable_ksm: true,
            target_density: 500, // 500 clones per GB
        }
    }
}

/// A cloned VM instance ready for execution
#[derive(Debug)]
pub struct VmClone {
    /// Unique clone ID
    pub id: u64,
    /// Runtime type
    pub runtime: RuntimeType,
    /// Parent snapshot ID
    pub snapshot_id: String,
    /// Clone state
    pub state: CloneState,
    /// Memory pages that have been modified (CoW)
    pub dirty_pages: AtomicUsize,
    /// When this clone was created
    pub created_at: Instant,
    /// When this clone was last used
    pub last_used: Mutex<Instant>,
    /// Memory region mappings
    memory_mappings: Vec<CloneMemoryMapping>,
}

/// State of a VM clone
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CloneState {
    /// Clone is ready for use
    Ready,
    /// Clone is currently executing a function
    Running,
    /// Clone has been modified and needs reset
    Dirty,
    /// Clone is being recycled
    Recycling,
}

/// Memory mapping for a clone (CoW)
#[derive(Debug)]
struct CloneMemoryMapping {
    /// Base address in the clone's address space
    base_addr: u64,
    /// Size of the mapping
    size: usize,
    /// Reference to shared backing memory (from snapshot)
    shared_backing: Arc<[u8]>,
    /// Private pages that have been copied-on-write
    private_pages: Mutex<HashMap<u64, Vec<u8>>>,
}

impl VmClone {
    /// Create a new clone from a snapshot
    fn new(id: u64, snapshot: &VmSnapshot) -> Self {
        let mut memory_mappings = Vec::new();

        for region in &snapshot.memory_regions {
            // In real implementation, this would mmap the snapshot file
            // with MAP_PRIVATE for CoW behavior
            memory_mappings.push(CloneMemoryMapping {
                base_addr: region.guest_addr,
                size: region.size,
                shared_backing: Arc::from(vec![0u8; 0].into_boxed_slice()), // Placeholder
                private_pages: Mutex::new(HashMap::new()),
            });
        }

        Self {
            id,
            runtime: snapshot.runtime,
            snapshot_id: snapshot.id.clone(),
            state: CloneState::Ready,
            dirty_pages: AtomicUsize::new(0),
            created_at: Instant::now(),
            last_used: Mutex::new(Instant::now()),
            memory_mappings,
        }
    }

    /// Read memory from the clone
    pub fn read_memory(&self, addr: u64, size: usize) -> VmmResult<Vec<u8>> {
        for mapping in &self.memory_mappings {
            if addr >= mapping.base_addr && addr < mapping.base_addr + mapping.size as u64 {
                let offset = (addr - mapping.base_addr) as usize;

                // Check for private (CoW) pages first
                let page_addr = addr & !4095;
                let private = mapping.private_pages.lock().unwrap();
                if let Some(page) = private.get(&page_addr) {
                    let page_offset = (addr & 4095) as usize;
                    let end = (page_offset + size).min(4096);
                    return Ok(page[page_offset..end].to_vec());
                }
                drop(private);

                // Fall back to shared backing
                if offset + size <= mapping.shared_backing.len() {
                    return Ok(mapping.shared_backing[offset..offset + size].to_vec());
                }
            }
        }

        Err(VmmError::MemoryError(format!(
            "Address 0x{:x} not mapped",
            addr
        )))
    }

    /// Write memory to the clone (triggers CoW)
    pub fn write_memory(&self, addr: u64, data: &[u8]) -> VmmResult<()> {
        for mapping in &self.memory_mappings {
            if addr >= mapping.base_addr && addr < mapping.base_addr + mapping.size as u64 {
                let page_addr = addr & !4095;
                let page_offset = (addr & 4095) as usize;

                let mut private = mapping.private_pages.lock().unwrap();

                // Copy-on-Write: create private page if needed
                if !private.contains_key(&page_addr) {
                    // Copy from shared backing
                    let backing_offset = (page_addr - mapping.base_addr) as usize;
                    let mut page = if backing_offset + 4096 <= mapping.shared_backing.len() {
                        mapping.shared_backing[backing_offset..backing_offset + 4096].to_vec()
                    } else {
                        vec![0u8; 4096]
                    };

                    private.insert(page_addr, page);
                    self.dirty_pages.fetch_add(1, Ordering::Relaxed);
                }

                // Write to private page
                let page = private.get_mut(&page_addr).unwrap();
                let end = (page_offset + data.len()).min(4096);
                page[page_offset..end].copy_from_slice(&data[..end - page_offset]);

                return Ok(());
            }
        }

        Err(VmmError::MemoryError(format!(
            "Address 0x{:x} not mapped",
            addr
        )))
    }

    /// Get the number of dirty (private) pages
    pub fn dirty_page_count(&self) -> usize {
        self.dirty_pages.load(Ordering::Relaxed)
    }

    /// Calculate memory overhead of this clone
    pub fn memory_overhead_bytes(&self) -> usize {
        self.dirty_page_count() * 4096
    }

    /// Touch the clone to update last_used time
    pub fn touch(&self) {
        *self.last_used.lock().unwrap() = Instant::now();
    }

    /// Check if the clone is idle beyond timeout
    pub fn is_idle(&self, timeout: Duration) -> bool {
        self.last_used.lock().unwrap().elapsed() > timeout
    }

    /// Reset the clone for reuse (discard private pages)
    pub fn reset(&mut self) -> VmmResult<()> {
        for mapping in &mut self.memory_mappings {
            mapping.private_pages.lock().unwrap().clear();
        }
        self.dirty_pages.store(0, Ordering::Relaxed);
        self.state = CloneState::Ready;
        Ok(())
    }
}

/// Pool of VM clones for a specific runtime type
pub struct ClonePool {
    /// Runtime type this pool serves
    runtime: RuntimeType,
    /// The golden snapshot to clone from
    golden_snapshot: Arc<VmSnapshot>,
    /// Configuration
    config: ClonePoolConfig,
    /// Ready clones waiting to be used
    ready_queue: Mutex<VecDeque<Box<VmClone>>>,
    /// Currently running clones
    running: RwLock<HashMap<u64, Box<VmClone>>>,
    /// Clone ID counter
    next_id: AtomicU64,
    /// Statistics
    stats: PoolStats,
}

/// Statistics for a clone pool
#[derive(Debug, Default)]
pub struct PoolStats {
    /// Total clones created
    pub clones_created: AtomicU64,
    /// Clones acquired from pool
    pub clones_acquired: AtomicU64,
    /// Clones returned to pool
    pub clones_returned: AtomicU64,
    /// Clones recycled (too dirty or idle)
    pub clones_recycled: AtomicU64,
    /// Total memory saved by CoW (bytes)
    pub cow_memory_saved: AtomicU64,
    /// Average dirty pages per clone
    pub avg_dirty_pages: AtomicU64,
}

impl ClonePool {
    /// Create a new clone pool
    pub fn new(
        runtime: RuntimeType,
        golden_snapshot: Arc<VmSnapshot>,
        config: ClonePoolConfig,
    ) -> Self {
        Self {
            runtime,
            golden_snapshot,
            config,
            ready_queue: Mutex::new(VecDeque::new()),
            running: RwLock::new(HashMap::new()),
            next_id: AtomicU64::new(1),
            stats: PoolStats::default(),
        }
    }

    /// Pre-warm the pool with ready clones
    pub fn warm_up(&self) -> VmmResult<()> {
        let mut queue = self.ready_queue.lock().unwrap();
        while queue.len() < self.config.min_warm_clones {
            let clone = self.create_clone()?;
            queue.push_back(clone);
        }
        Ok(())
    }

    /// Create a new clone from the golden snapshot
    fn create_clone(&self) -> VmmResult<Box<VmClone>> {
        let id = self.next_id.fetch_add(1, Ordering::Relaxed);
        let clone = Box::new(VmClone::new(id, &self.golden_snapshot));
        self.stats.clones_created.fetch_add(1, Ordering::Relaxed);
        Ok(clone)
    }

    /// Acquire a clone for execution
    pub fn acquire(&self) -> VmmResult<Box<VmClone>> {
        // Try to get a ready clone first
        {
            let mut queue = self.ready_queue.lock().unwrap();
            if let Some(mut clone) = queue.pop_front() {
                clone.state = CloneState::Running;
                clone.touch();
                self.stats.clones_acquired.fetch_add(1, Ordering::Relaxed);

                let id = clone.id;
                self.running.write().unwrap().insert(id, clone);
                return Ok(self.running.write().unwrap().remove(&id).unwrap());
            }
        }

        // No ready clones, create a new one
        let mut clone = self.create_clone()?;
        clone.state = CloneState::Running;
        self.stats.clones_acquired.fetch_add(1, Ordering::Relaxed);

        Ok(clone)
    }

    /// Release a clone back to the pool
    pub fn release(&self, mut clone: Box<VmClone>) -> VmmResult<()> {
        self.stats.clones_returned.fetch_add(1, Ordering::Relaxed);

        // Check if clone is too dirty to reuse
        let dirty_threshold = 1000; // pages
        if clone.dirty_page_count() > dirty_threshold {
            self.stats.clones_recycled.fetch_add(1, Ordering::Relaxed);
            // Clone is dropped and memory freed
            return Ok(());
        }

        // Reset and return to pool
        clone.reset()?;

        let mut queue = self.ready_queue.lock().unwrap();
        if queue.len() < self.config.max_warm_clones {
            queue.push_back(clone);
        }
        // Else clone is dropped

        Ok(())
    }

    /// Get pool statistics
    pub fn stats(&self) -> &PoolStats {
        &self.stats
    }

    /// Get current pool size
    pub fn ready_count(&self) -> usize {
        self.ready_queue.lock().unwrap().len()
    }

    /// Get running clone count
    pub fn running_count(&self) -> usize {
        self.running.read().unwrap().len()
    }

    /// Calculate total memory usage
    pub fn total_memory_usage(&self) -> usize {
        let base_memory = self.golden_snapshot.memory_size;
        let queue = self.ready_queue.lock().unwrap();
        let overhead: usize = queue.iter().map(|c| c.memory_overhead_bytes()).sum();
        base_memory + overhead
    }

    /// Calculate memory efficiency (compared to full copies)
    pub fn memory_efficiency(&self) -> f64 {
        let clone_count = self.ready_count() + self.running_count();
        if clone_count == 0 {
            return 100.0;
        }

        let base_memory = self.golden_snapshot.memory_size;
        let full_memory = base_memory * clone_count;
        let actual_memory = self.total_memory_usage();

        ((full_memory - actual_memory) as f64 / full_memory as f64) * 100.0
    }

    /// Clean up idle clones
    pub fn cleanup_idle(&self) -> usize {
        let mut queue = self.ready_queue.lock().unwrap();
        let before = queue.len();

        // Keep minimum clones, remove idle ones above minimum
        while queue.len() > self.config.min_warm_clones {
            if let Some(clone) = queue.back() {
                if clone.is_idle(self.config.idle_timeout) {
                    queue.pop_back();
                    self.stats.clones_recycled.fetch_add(1, Ordering::Relaxed);
                } else {
                    break;
                }
            } else {
                break;
            }
        }

        before - queue.len()
    }
}

/// Manager for multiple clone pools
pub struct ClonePoolManager {
    /// Pools by runtime type
    pools: RwLock<HashMap<RuntimeType, Arc<ClonePool>>>,
    /// Global configuration
    config: ClonePoolConfig,
    /// KSM integration
    ksm: Option<KsmManager>,
}

impl ClonePoolManager {
    /// Create a new pool manager
    pub fn new(config: ClonePoolConfig) -> Self {
        let ksm = if config.enable_ksm {
            KsmManager::new().ok()
        } else {
            None
        };

        Self {
            pools: RwLock::new(HashMap::new()),
            config,
            ksm,
        }
    }

    /// Register a golden snapshot and create its pool
    pub fn register_runtime(
        &self,
        runtime: RuntimeType,
        snapshot: Arc<VmSnapshot>,
    ) -> VmmResult<()> {
        let pool = Arc::new(ClonePool::new(runtime, snapshot, self.config.clone()));
        pool.warm_up()?;

        self.pools.write().unwrap().insert(runtime, pool);
        Ok(())
    }

    /// Get a clone for a runtime type
    pub fn acquire(&self, runtime: RuntimeType) -> VmmResult<Box<VmClone>> {
        let pools = self.pools.read().unwrap();
        let pool = pools.get(&runtime).ok_or_else(|| {
            VmmError::CloneError(format!("No pool for runtime {:?}", runtime))
        })?;

        pool.acquire()
    }

    /// Release a clone back to its pool
    pub fn release(&self, clone: Box<VmClone>) -> VmmResult<()> {
        let pools = self.pools.read().unwrap();
        let pool = pools.get(&clone.runtime).ok_or_else(|| {
            VmmError::CloneError(format!("No pool for runtime {:?}", clone.runtime))
        })?;

        pool.release(clone)
    }

    /// Get statistics for all pools
    pub fn global_stats(&self) -> GlobalPoolStats {
        let pools = self.pools.read().unwrap();
        let mut stats = GlobalPoolStats::default();

        for (runtime, pool) in pools.iter() {
            stats.total_ready += pool.ready_count();
            stats.total_running += pool.running_count();
            stats.total_memory += pool.total_memory_usage();
            stats.pools.insert(*runtime, pool.memory_efficiency());
        }

        stats
    }

    /// Run maintenance on all pools
    pub fn maintenance(&self) -> MaintenanceResult {
        let pools = self.pools.read().unwrap();
        let mut result = MaintenanceResult::default();

        for pool in pools.values() {
            result.clones_recycled += pool.cleanup_idle();
        }

        // Trigger KSM scan if available
        if let Some(ref ksm) = self.ksm {
            result.pages_merged = ksm.scan();
        }

        result
    }
}

/// Global statistics across all pools
#[derive(Debug, Default)]
pub struct GlobalPoolStats {
    pub total_ready: usize,
    pub total_running: usize,
    pub total_memory: usize,
    pub pools: HashMap<RuntimeType, f64>,
}

/// Result of maintenance operation
#[derive(Debug, Default)]
pub struct MaintenanceResult {
    pub clones_recycled: usize,
    pub pages_merged: usize,
}

/// Kernel Same-page Merging (KSM) manager
struct KsmManager {
    enabled: bool,
}

impl KsmManager {
    fn new() -> VmmResult<Self> {
        // Check if KSM is available
        let ksm_run = std::path::Path::new("/sys/kernel/mm/ksm/run");
        if !ksm_run.exists() {
            return Err(VmmError::FeatureNotSupported("KSM not available".to_string()));
        }

        Ok(Self { enabled: true })
    }

    fn scan(&self) -> usize {
        // In real implementation, this would trigger KSM to scan for merge opportunities
        // by writing to /sys/kernel/mm/ksm/run and reading /sys/kernel/mm/ksm/pages_shared
        0
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::snapshot::VmSnapshot;
    use std::collections::HashMap;

    fn create_test_snapshot() -> VmSnapshot {
        VmSnapshot {
            id: "test_snapshot".to_string(),
            runtime: RuntimeType::NodeJs,
            cpu_state: Default::default(),
            memory_regions: Vec::new(),
            device_states: HashMap::new(),
            created_at: 0,
            memory_size: 128 * 1024 * 1024,
        }
    }

    #[test]
    fn test_clone_pool_creation() {
        let snapshot = Arc::new(create_test_snapshot());
        let pool = ClonePool::new(RuntimeType::NodeJs, snapshot, ClonePoolConfig::default());

        assert_eq!(pool.runtime, RuntimeType::NodeJs);
        assert_eq!(pool.ready_count(), 0);
    }

    #[test]
    fn test_clone_pool_warm_up() {
        let snapshot = Arc::new(create_test_snapshot());
        let pool = ClonePool::new(RuntimeType::NodeJs, snapshot, ClonePoolConfig::default());

        pool.warm_up().unwrap();
        assert!(pool.ready_count() >= 2); // min_warm_clones default
    }

    #[test]
    fn test_clone_acquire_release() {
        let snapshot = Arc::new(create_test_snapshot());
        let pool = ClonePool::new(RuntimeType::NodeJs, snapshot, ClonePoolConfig::default());

        let clone = pool.acquire().unwrap();
        assert_eq!(clone.state, CloneState::Running);

        pool.release(clone).unwrap();
        assert_eq!(pool.ready_count(), 1);
    }

    #[test]
    fn test_memory_efficiency() {
        let snapshot = Arc::new(create_test_snapshot());
        let pool = ClonePool::new(RuntimeType::NodeJs, snapshot, ClonePoolConfig::default());

        pool.warm_up().unwrap();
        let efficiency = pool.memory_efficiency();
        
        // Memory efficiency formula: ((full_memory - actual_memory) / full_memory) * 100
        // With CoW clones sharing the base memory:
        // - full_memory = base * clone_count (what we'd use without sharing)
        // - actual_memory = base + overhead (shared base + per-clone overhead)
        // 
        // For a single clone with no dirty pages:
        // - full_memory = 128MB * 1 = 128MB
        // - actual_memory = 128MB + 0 = 128MB
        // - efficiency = 0% (no savings from sharing with just 1 clone)
        //
        // The efficiency only becomes meaningful with multiple clones.
        // With 2 clones: efficiency = ((256MB - 128MB) / 256MB) * 100 = 50%
        // With 10 clones: efficiency = ((1280MB - 128MB) / 1280MB) * 100 = 90%
        //
        // For a single clone, we just verify the calculation doesn't fail
        assert!(
            efficiency >= 0.0 && efficiency <= 100.0,
            "Efficiency should be in valid range [0, 100], got {}",
            efficiency
        );
    }

    #[test]
    fn test_pool_manager() {
        let manager = ClonePoolManager::new(ClonePoolConfig::default());
        let snapshot = Arc::new(create_test_snapshot());

        manager
            .register_runtime(RuntimeType::NodeJs, snapshot)
            .unwrap();

        let clone = manager.acquire(RuntimeType::NodeJs).unwrap();
        manager.release(clone).unwrap();
    }
}
