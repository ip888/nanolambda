# MicroVM Isolation - Realistic Development Plan

**Date:** December 28, 2025  
**Current Status:** Process-based isolation (production-ready)  
**Target:** Add microVM isolation as optional security layer  
**Estimated Timeline:** 4-6 weeks (1 developer full-time)

---

## 🎯 Executive Summary

NanoLambda currently uses **process-based isolation** which is:
- ✅ Production-ready and battle-tested
- ✅ Fast: ~0ms warm starts, ~32ms cold starts
- ✅ Simple: No kernel dependencies
- ✅ Sufficient for 95% of use cases

**MicroVM isolation would add:**
- 🔒 Hardware-backed security (KVM)
- 🏢 Multi-tenant isolation (untrusted code)
- 📊 Marketing differentiation ("VM-level security")
- 💰 Premium pricing tier justification

**Trade-offs:**
- ⚠️ Cold start overhead: +50-100ms (firecracker boot)
- ⚠️ Memory overhead: ~128MB per microVM
- ⚠️ Complexity: KVM, device management, kernel builds
- ⚠️ Linux-only: No Windows/macOS support

---

## 📊 Current Architecture

```
┌─────────────────────────────────────┐
│        API Server (Axum)            │
│  - Request handling                 │
│  - Authentication                   │
│  - Routing                          │
└──────────────┬──────────────────────┘
               │
               ▼
┌─────────────────────────────────────┐
│    Runtime Layer (Trait-based)      │
│  - PythonExecutor                   │
│  - NodeJSExecutor                   │
│  - Process pooling                  │
│  - Metrics collection               │
└──────────────┬──────────────────────┘
               │
               ▼
┌─────────────────────────────────────┐
│      Process Isolation              │
│  - Linux processes                  │
│  - Resource limits (ulimit)         │
│  - Namespace isolation (optional)   │
└─────────────────────────────────────┘
```

---

## 🎯 Target Architecture (with microVM)

```
┌─────────────────────────────────────┐
│        API Server (Axum)            │
└──────────────┬──────────────────────┘
               │
               ▼
┌─────────────────────────────────────┐
│    Runtime Layer (Trait-based)      │
│  - PythonExecutor                   │
│  - NodeJSExecutor                   │
│  - ProcessPoolExecutor (current)    │
│  - MicroVMExecutor (new) ◄─────────┐
└──────────────┬──────────────────────┘│
               │                       │
               ▼                       │
┌─────────────────────────────────────┐│
│   Isolation Strategy Selection      ││
│  - Environment variable             ││
│  - Per-function config              ││
│  - Tier-based (free=process)        ││
└──────────────┬──────────────────────┘│
               │                       │
       ┌───────┴────────┐             │
       ▼                ▼             │
┌─────────────┐  ┌──────────────────┐ │
│  Process    │  │  MicroVM Layer   │ │
│  Isolation  │  │  (new crate)     │◄┘
│  (current)  │  │  - Firecracker   │
│             │  │  - Jailer        │
│             │  │  - VMM mgmt      │
└─────────────┘  └──────────────────┘
```

---

## 📋 Development Phases

### **Phase 1: Research & Setup (1 week)**

#### Week 1: Environment & Proof of Concept

**Goals:**
- ✅ Verify KVM works in dev environment
- ✅ Run Firecracker manually
- ✅ Boot minimal Linux guest
- ✅ Understand Firecracker API

**Tasks:**
1. **Day 1-2: Environment Setup**
   ```bash
   # Install Firecracker
   release_url="https://github.com/firecracker-microvm/firecracker/releases"
   latest=$(basename $(curl -fsSLI -o /dev/null -w  %{url_effective} ${release_url}/latest))
   arch=`uname -m`
   curl -L ${release_url}/download/${latest}/firecracker-${latest}-${arch}.tgz | tar -xz
   
   # Verify KVM access
   [ -r /dev/kvm ] && [ -w /dev/kvm ] && echo "OK" || echo "FAIL"
   
   # Test Firecracker
   ./firecracker --version
   ```

2. **Day 3-4: Boot Test VM**
   - Download Alpine Linux kernel + rootfs
   - Write Firecracker config JSON
   - Boot VM manually, execute command
   - Understand vsock communication

3. **Day 5: Document Findings**
   - Performance baseline (boot time, memory)
   - API surface understanding
   - Security model analysis
   - Decision: Firecracker vs alternatives

**Deliverables:**
- `docs/MICROVM_POC_RESULTS.md` - What works, what doesn't
- `scripts/test-firecracker.sh` - Manual test script
- Decision doc: Continue or pivot?

---

### **Phase 2: Core MicroVM Crate (2 weeks)**

#### Week 2-3: Build `nanolambda-microvm` Crate

**Goals:**
- ✅ Rust crate for Firecracker lifecycle
- ✅ VM configuration management
- ✅ Guest image building
- ✅ Basic function execution

**Architecture:**
```
crates/microvm/
├── Cargo.toml
├── src/
│   ├── lib.rs                  # Public API
│   ├── firecracker.rs          # Firecracker process mgmt
│   ├── config.rs               # VM configuration
│   ├── guest/                  # Guest image tools
│   │   ├── kernel.rs           # Kernel management
│   │   ├── rootfs.rs           # Root filesystem
│   │   └── init.rs             # Guest init process
│   ├── jailer.rs               # Security isolation
│   ├── vsock.rs                # Guest communication
│   └── error.rs                # Error types
├── tests/
│   ├── basic_vm_test.rs
│   └── function_exec_test.rs
└── guest/                      # Guest filesystem assets
    ├── kernel/                 # Linux kernel binaries
    ├── rootfs/                 # Root filesystem images
    └── init/                   # Init scripts
```

**Key APIs:**

```rust
// src/lib.rs
pub struct MicroVM {
    id: String,
    config: VMConfig,
    process: Option<Child>,
    vsock_path: PathBuf,
}

impl MicroVM {
    pub async fn new(config: VMConfig) -> Result<Self>;
    pub async fn start(&mut self) -> Result<()>;
    pub async fn execute(&self, code: &str, event: &Value) -> Result<Value>;
    pub async fn stop(&mut self) -> Result<()>;
}

// src/config.rs
pub struct VMConfig {
    pub vcpu_count: u8,
    pub mem_size_mib: usize,
    pub kernel_image_path: PathBuf,
    pub rootfs_image_path: PathBuf,
    pub runtime: Runtime,  // Python, Node.js, etc.
}
```

**Implementation Tasks:**

1. **Day 1-3: Firecracker Integration**
   - Spawn Firecracker process
   - Generate config JSON
   - API socket communication
   - Process lifecycle management

2. **Day 4-6: Guest Image Builder**
   - Download/build Alpine Linux kernel
   - Create minimal rootfs (Alpine base)
   - Add Python/Node.js to rootfs
   - Build init script (launches function)

3. **Day 7-9: Communication Layer**
   - vsock setup
   - JSON-based IPC (like current process IPC)
   - Stdin/stdout over vsock
   - Error handling

4. **Day 10-12: Testing**
   - Unit tests for each component
   - Integration tests (boot → execute → shutdown)
   - Performance benchmarks
   - Memory leak tests

**Deliverables:**
- Working `nanolambda-microvm` crate
- 20+ passing tests
- Performance report: boot time, overhead
- `docs/MICROVM_CRATE_GUIDE.md`

---

### **Phase 3: Runtime Integration (1 week)**

#### Week 4: Integrate with Runtime Layer

**Goals:**
- ✅ `MicroVMExecutor` implements `Runtime` trait
- ✅ Drop-in replacement for process executors
- ✅ Configuration for isolation mode

**New File: `crates/microvm/src/executor.rs`**

```rust
use nanolambda_runtime::{Runtime, ExecutionResult, FunctionConfig};
use crate::MicroVM;

pub struct MicroVMExecutor {
    vm_pool: Option<VecDeque<MicroVM>>,  // Optional pooling
    config: MicroVMConfig,
}

impl Runtime for MicroVMExecutor {
    fn execute(
        &mut self,
        config: &FunctionConfig,
        event: &serde_json::Value,
    ) -> Result<ExecutionResult> {
        // Get VM from pool or create new
        let mut vm = self.get_or_create_vm(config)?;
        
        // Execute in VM
        let result = vm.execute(&config.code, event).await?;
        
        // Return VM to pool or destroy
        self.return_or_destroy_vm(vm)?;
        
        Ok(result)
    }
    
    fn get_runtime_info(&self) -> RuntimeInfo {
        RuntimeInfo {
            name: "microvm".to_string(),
            version: env!("CARGO_PKG_VERSION").to_string(),
            isolation: IsolationType::MicroVM,
        }
    }
}
```

**API Server Integration:**

```rust
// crates/api-server/src/lib.rs
pub struct ApiServer {
    storage: StorageManager,
    python_executor: Box<dyn Runtime>,  // Could be Process or MicroVM
    nodejs_executor: Box<dyn Runtime>,
}

impl ApiServer {
    pub fn new(db_path: &str, isolation_mode: IsolationMode) -> Result<Self> {
        let python_executor: Box<dyn Runtime> = match isolation_mode {
            IsolationMode::Process => Box::new(PythonExecutor::new()?),
            IsolationMode::MicroVM => Box::new(MicroVMExecutor::new(Language::Python)?),
        };
        
        // Same for Node.js...
        
        Ok(ApiServer { storage, python_executor, nodejs_executor })
    }
}
```

**Configuration:**

```bash
# Environment variable
export NANOLAMBDA_ISOLATION=microvm  # or "process" (default)

# Or per-function in database
UPDATE functions SET isolation_mode = 'microvm' WHERE name = 'sensitive-function';

# Or tier-based
# Free tier: process isolation
# Pro tier: microVM isolation
```

**Tasks:**

1. **Day 1-2: Implement MicroVMExecutor**
   - Implement `Runtime` trait
   - VM lifecycle management
   - Error handling

2. **Day 3-4: API Server Integration**
   - Add isolation mode config
   - Update function handlers
   - Backward compatibility testing

3. **Day 5: End-to-End Testing**
   - Test both isolation modes
   - Performance comparison
   - Failure scenarios

**Deliverables:**
- Working microVM executor
- API server supports both modes
- Integration tests passing
- Performance comparison report

---

### **Phase 4: Production Hardening (1-2 weeks)**

#### Week 5-6: Security, Performance, Documentation

**Goals:**
- ✅ Jailer integration (security)
- ✅ Resource limits
- ✅ Monitoring & metrics
- ✅ Production documentation

**Security Tasks:**

1. **Jailer Setup**
   ```rust
   // Use Firecracker's jailer for additional isolation
   pub struct JailedVM {
       jailer_process: Child,
       chroot_path: PathBuf,
       netns: Option<String>,
   }
   ```

2. **Resource Limits**
   - Rate limiting (VMs per user)
   - Memory quotas
   - CPU quotas (cgroups)
   - Network isolation (if needed)

3. **Security Audit**
   - Review vsock permissions
   - Test escape scenarios
   - Validate jailer config
   - Document threat model

**Performance Tasks:**

1. **VM Pooling**
   - Pre-warm VMs
   - Pool management
   - Metrics collection

2. **Optimization**
   - Reduce boot time (snapshot/restore)
   - Minimize memory overhead
   - Optimize guest image size

3. **Benchmarking**
   - Cold start times
   - Warm start times
   - Throughput testing
   - Memory usage profiling

**Documentation Tasks:**

1. **User Documentation**
   - How to enable microVM mode
   - Performance expectations
   - Limitations
   - Troubleshooting

2. **Operator Documentation**
   - KVM setup requirements
   - Host configuration
   - Monitoring
   - Capacity planning

3. **Developer Documentation**
   - Architecture overview
   - Adding new runtimes
   - Testing guide
   - Contributing guide

**Deliverables:**
- Security-hardened implementation
- Performance benchmarks
- Complete documentation
- Production deployment guide

---

## 📊 Implementation Checklist

### Phase 1: Research (Week 1)
- [ ] Verify KVM works in dev environment
- [ ] Install Firecracker
- [ ] Boot test VM manually
- [ ] Understand Firecracker API
- [ ] Performance baseline measurements
- [ ] Document POC results
- [ ] Go/No-Go decision

### Phase 2: Core Crate (Weeks 2-3)
- [ ] Create `crates/microvm/` structure
- [ ] Implement Firecracker process management
- [ ] Build guest kernel + rootfs
- [ ] Implement vsock communication
- [ ] Function execution working
- [ ] 20+ unit tests
- [ ] Integration tests
- [ ] Performance benchmarks

### Phase 3: Runtime Integration (Week 4)
- [ ] Implement `MicroVMExecutor`
- [ ] Implement `Runtime` trait
- [ ] API server integration
- [ ] Configuration system
- [ ] Backward compatibility tests
- [ ] End-to-end tests
- [ ] Performance comparison

### Phase 4: Production (Weeks 5-6)
- [ ] Jailer integration
- [ ] Resource limiting
- [ ] Security audit
- [ ] VM pooling
- [ ] Optimization pass
- [ ] Monitoring integration
- [ ] User documentation
- [ ] Operator documentation
- [ ] Deployment guide

---

## 🎯 Success Criteria

### Functional Requirements
- ✅ Both isolation modes work side-by-side
- ✅ Zero breaking changes to API
- ✅ Configuration-driven selection
- ✅ All existing tests pass

### Performance Requirements
- ✅ Cold start: <150ms (target: <100ms)
- ✅ Warm start: <10ms (with pooling)
- ✅ Memory overhead: <150MB per VM
- ✅ Throughput: >100 req/s per host

### Security Requirements
- ✅ VM escape attempts fail
- ✅ Resource limits enforced
- ✅ Network isolation working
- ✅ Jailer properly configured

### Documentation Requirements
- ✅ API documentation complete
- ✅ Deployment guide tested
- ✅ Troubleshooting guide
- ✅ Performance tuning guide

---

## 💰 Cost-Benefit Analysis

### Development Cost
- **Time:** 4-6 weeks (1 developer)
- **Complexity:** High (kernel, VMs, security)
- **Maintenance:** Medium (Firecracker updates, kernel CVEs)

### Benefits
- **Security:** Hardware isolation for untrusted code
- **Marketing:** "VM-level security" positioning
- **Pricing:** Justify premium tier ($X/month extra)
- **Compliance:** Some industries require VM isolation

### Risks
- **Complexity:** More moving parts = more failure modes
- **Performance:** Cold starts increase 3-4x
- **Dependencies:** KVM requirement limits deployment
- **Maintenance:** Kernel/Firecracker security updates

### Recommendation
**Implement as OPTIONAL feature:**
- Default: Process isolation (current, fast, simple)
- Optional: MicroVM (for security-sensitive workloads)
- User choice via config or pricing tier

---

## 🚀 Alternative: Faster Path

If 4-6 weeks is too long, consider:

### Option A: Firecracker-Containerd
Use existing `firecracker-containerd` project:
- **Pros:** Pre-built, maintained by AWS
- **Cons:** Less control, heavier weight
- **Timeline:** 1-2 weeks integration

### Option B: Kata Containers
Use Kata Containers (lightweight VMs):
- **Pros:** OCI-compatible, well-tested
- **Cons:** Heavier than Firecracker
- **Timeline:** 1-2 weeks integration

### Option C: gVisor
User-space kernel (not VM, but strong isolation):
- **Pros:** No KVM requirement, fast
- **Cons:** Not "true VM," Linux-only
- **Timeline:** 1 week integration

---

## 📚 Resources

### Essential Reading
- [Firecracker Design](https://github.com/firecracker-microvm/firecracker/blob/main/docs/design.md)
- [Firecracker API](https://github.com/firecracker-microvm/firecracker/blob/main/src/api_server/swagger/firecracker.yaml)
- [rust-vmm](https://github.com/rust-vmm) - Reusable VMM components
- [AWS Lambda Firecracker](https://aws.amazon.com/blogs/aws/firecracker-lightweight-virtualization-for-serverless-computing/)

### Example Projects
- [Firecracker Go SDK](https://github.com/firecracker-microvm/firecracker-go-sdk)
- [Fly.io Firecracker](https://fly.io/blog/sandboxing-and-workload-isolation/) - How they use it
- [Weave Ignite](https://github.com/weaveworks/ignite) - Firecracker with Docker UX

### Technical Guides
- [Building Alpine Linux for Firecracker](https://github.com/firecracker-microvm/firecracker/blob/main/docs/rootfs-and-kernel-setup.md)
- [Jailer Guide](https://github.com/firecracker-microvm/firecracker/blob/main/docs/jailer.md)
- [Performance Tuning](https://github.com/firecracker-microvm/firecracker/blob/main/docs/prod-host-setup.md)

---

## 🎯 Next Steps

1. **Review this plan** with team/stakeholders
2. **Verify KVM access** in target deployment environment
3. **Decide priority:** Now vs later?
4. **Start Phase 1** if approved (1 week POC)
5. **Re-evaluate** after POC results

---

## 📝 Notes

- **Current system is production-ready** - Don't rush this
- **MicroVM is enhancement**, not requirement
- **Test in dev first** - KVM issues are common
- **Consider alternatives** if Firecracker is too complex
- **Budget 2x time** for unexpected issues (kernel bugs, etc.)

---

**Realistic Timeline:** 6-8 weeks including testing and documentation  
**Minimum Viable:** 3-4 weeks for basic working implementation  
**Recommended:** Start with 1-week POC to validate approach
