# MicroVM Implementation Plan: Real Firecracker Integration

**Status:** APPROVED FOR IMPLEMENTATION  
**Timeline:** 12-16 weeks  
**Goal:** Production-ready microVM isolation for multi-tenant security

---

## 🎯 **Executive Decision: Separate Crate or Project?**

### **RECOMMENDED: Separate Crate in Monorepo** ✅

**Architecture:**
```
nanolambda/
├─ crates/
│   ├─ runtime/              # Existing: Process pool execution
│   ├─ runtime-microvm/      # NEW: MicroVM-based execution
│   ├─ api-server/           # Existing: Routes to either runtime
│   ├─ storage/              # Existing: Shared
│   └─ scheduler/            # Existing: Shared
└─ Cargo.toml                # Workspace
```

### **Why NOT a Separate Project:**

❌ **Code Duplication:**
- Would duplicate storage, API server, scheduler logic
- Double maintenance burden

❌ **Integration Complexity:**
- Harder to keep APIs in sync
- Users need two separate deployments

❌ **Fragmentation:**
- Splits community
- Harder to share improvements

### **Why YES to Separate Crate:**

✅ **Clean Separation:**
- MicroVM code doesn't pollute main runtime
- Can be enabled/disabled at compile time
- Different dependency tree (KVM, Firecracker)

✅ **Optional Feature:**
```toml
[dependencies]
nanolambda-runtime = "0.1"              # Default: Process pool
nanolambda-runtime-microvm = "0.1"      # Optional: MicroVMs
```

✅ **Shared Infrastructure:**
- Same API server
- Same storage layer
- Same scheduler
- Only execution layer differs

✅ **User Choice:**
```yaml
# config.yaml
runtime:
  type: "process"      # Default: Fast, simple
  # OR
  type: "microvm"      # Enterprise: Secure, isolated
```

---

## 📋 **Project Structure**

### **New Crate: `crates/runtime-microvm/`**

```
crates/runtime-microvm/
├─ Cargo.toml
├─ src/
│   ├─ lib.rs                    # Public API (same interface as runtime)
│   ├─ executor.rs               # MicroVMExecutor trait impl
│   ├─ firecracker.rs            # Firecracker wrapper
│   ├─ vmm/
│   │   ├─ mod.rs
│   │   ├─ vm.rs                 # VM lifecycle management
│   │   ├─ network.rs            # virtio-net setup
│   │   ├─ block.rs              # virtio-blk for function code
│   │   └─ vsock.rs              # virtio-vsock for IPC
│   ├─ guest/
│   │   ├─ init.rs               # Minimal init process
│   │   ├─ python_runtime.sh     # Guest Python setup
│   │   └─ node_runtime.sh       # Guest Node.js setup
│   ├─ snapshots/
│   │   ├─ manager.rs            # Snapshot/restore for fast boot
│   │   └─ cache.rs              # Pre-warmed VM cache
│   └─ pool.rs                   # MicroVM pool manager
├─ tests/
│   ├─ integration.rs
│   └─ security.rs
├─ guest-images/                 # Minimal Linux rootfs
│   ├─ build.sh                  # Build script for guest kernel
│   ├─ kernel-config             # Minimal kernel config
│   └─ rootfs/                   # Alpine-based minimal rootfs
└─ README.md
```

---

## 🏗️ **Implementation Roadmap**

### **Phase 1: Foundation (Weeks 1-3)**

#### **Week 1: Project Setup**

**Tasks:**
1. Create `crates/runtime-microvm/` crate
2. Add Firecracker as dependency
3. Define trait interface (compatible with existing Runtime)
4. Set up CI/CD for KVM testing

**Deliverables:**
```rust
// crates/runtime-microvm/src/lib.rs
use nanolambda_runtime::RuntimeTrait;

pub struct MicroVMExecutor {
    config: MicroVMConfig,
    pool: VmPool,
}

#[async_trait]
impl RuntimeTrait for MicroVMExecutor {
    async fn execute(
        &mut self,
        code: &str,
        event: Value,
        config: &FunctionConfig,
    ) -> Result<ExecutionResult>;
}
```

**Success Criteria:**
- [ ] Crate compiles
- [ ] Trait interface matches existing runtime
- [ ] Basic test harness works

---

#### **Week 2-3: Firecracker Integration**

**Tasks:**
1. Integrate Firecracker binary
2. Implement VM creation and boot
3. Build minimal guest kernel (Alpine Linux)
4. Create init system for guest

**Code Example:**
```rust
// crates/runtime-microvm/src/firecracker.rs
use std::process::{Command, Stdio};
use std::path::PathBuf;

pub struct FirecrackerVM {
    socket_path: PathBuf,
    pid: u32,
    config: VmConfig,
}

impl FirecrackerVM {
    pub fn new(config: VmConfig) -> Result<Self> {
        // 1. Create Unix socket for API
        let socket_path = temp_dir().join(format!("fc-{}.sock", uuid()));
        
        // 2. Start Firecracker daemon
        let child = Command::new("firecracker")
            .arg("--api-sock")
            .arg(&socket_path)
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()?;
        
        // 3. Configure via API
        let vm = Self {
            socket_path,
            pid: child.id(),
            config,
        };
        
        vm.configure_machine()?;
        vm.configure_boot_source()?;
        vm.configure_network()?;
        
        Ok(vm)
    }
    
    fn configure_machine(&self) -> Result<()> {
        let config = json!({
            "vcpu_count": self.config.vcpus,
            "mem_size_mib": self.config.memory_mb,
        });
        
        self.api_call("PUT", "/machine-config", config)
    }
    
    fn configure_boot_source(&self) -> Result<()> {
        let config = json!({
            "kernel_image_path": "/opt/nanolambda/vmlinux",
            "boot_args": "console=ttyS0 reboot=k panic=1 pci=off",
        });
        
        self.api_call("PUT", "/boot-source", config)
    }
    
    pub fn start(&self) -> Result<()> {
        self.api_call("PUT", "/actions", json!({"action_type": "InstanceStart"}))
    }
}
```

**Guest Kernel Build:**
```bash
# guest-images/build.sh
#!/bin/bash
set -e

# Download minimal kernel
wget https://cdn.kernel.org/pub/linux/kernel/v6.x/linux-6.1.tar.xz
tar xf linux-6.1.tar.xz
cd linux-6.1

# Minimal config for Firecracker
cat > .config <<EOF
CONFIG_64BIT=y
CONFIG_KVM_GUEST=y
CONFIG_VIRTIO=y
CONFIG_VIRTIO_PCI=y
CONFIG_VIRTIO_BLK=y
CONFIG_VIRTIO_NET=y
CONFIG_VIRTIO_CONSOLE=y
CONFIG_EXT4_FS=y
CONFIG_BINFMT_ELF=y
CONFIG_TTY=y
CONFIG_SERIAL_8250=y
CONFIG_SERIAL_8250_CONSOLE=y
# Disable everything else
EOF

make olddefconfig
make vmlinux -j$(nproc)

# Copy to output
cp vmlinux ../vmlinux
```

**Success Criteria:**
- [ ] Firecracker boots a VM
- [ ] Kernel loads successfully
- [ ] Can connect to VM via serial console
- [ ] VM shuts down cleanly

---

### **Phase 2: Guest Runtime (Weeks 4-6)**

#### **Week 4: Minimal Init System**

**Tasks:**
1. Create tiny init process (Rust)
2. Set up Python interpreter in guest
3. Implement function loading mechanism

**Guest Init Process:**
```rust
// crates/runtime-microvm/guest-init/src/main.rs
use std::process::Command;
use std::io::{Read, Write};
use std::os::unix::net::UnixStream;

fn main() {
    // 1. Mount essential filesystems
    mount_proc();
    mount_sys();
    mount_tmp();
    
    // 2. Set up networking
    configure_network();
    
    // 3. Start function runtime
    start_python_runtime();
}

fn start_python_runtime() {
    loop {
        // Read function code via virtio-vsock
        let request = read_from_vsock();
        
        // Execute Python
        let result = Command::new("python3")
            .arg("-c")
            .arg(&request.code)
            .output()
            .expect("Failed to execute");
        
        // Send response back
        write_to_vsock(&result);
    }
}
```

**Success Criteria:**
- [ ] Guest boots to init
- [ ] Python interpreter available
- [ ] Can execute simple Python code
- [ ] Results returned to host

---

#### **Week 5-6: IPC via virtio-vsock**

**Tasks:**
1. Implement virtio-vsock communication
2. Create request/response protocol
3. Add error handling and timeouts

**Host-to-Guest Communication:**
```rust
// crates/runtime-microvm/src/vsock.rs
use vsock::{VsockStream, VsockAddr};

pub struct VsockChannel {
    stream: VsockStream,
}

impl VsockChannel {
    pub fn connect(cid: u32, port: u32) -> Result<Self> {
        let addr = VsockAddr::new(cid, port);
        let stream = VsockStream::connect(&addr)?;
        Ok(Self { stream })
    }
    
    pub fn send_request(&mut self, request: ExecutionRequest) -> Result<()> {
        let json = serde_json::to_vec(&request)?;
        let len = (json.len() as u32).to_le_bytes();
        
        self.stream.write_all(&len)?;
        self.stream.write_all(&json)?;
        self.stream.flush()?;
        
        Ok(())
    }
    
    pub fn receive_response(&mut self) -> Result<ExecutionResult> {
        let mut len_bytes = [0u8; 4];
        self.stream.read_exact(&mut len_bytes)?;
        let len = u32::from_le_bytes(len_bytes);
        
        let mut buffer = vec![0u8; len as usize];
        self.stream.read_exact(&mut buffer)?;
        
        let result = serde_json::from_slice(&buffer)?;
        Ok(result)
    }
}
```

**Success Criteria:**
- [ ] Host can send code to guest
- [ ] Guest executes and returns results
- [ ] Error handling works
- [ ] Timeout protection works

---

### **Phase 3: Performance Optimization (Weeks 7-9)**

#### **Week 7: Snapshot/Restore for Fast Boot**

**Problem:** Cold boot takes 125ms  
**Solution:** Snapshot pre-booted VMs, restore in 5-10ms

```rust
// crates/runtime-microvm/src/snapshots/manager.rs
pub struct SnapshotManager {
    base_snapshots: HashMap<Runtime, PathBuf>,
}

impl SnapshotManager {
    pub fn create_base_snapshot(&self, runtime: Runtime) -> Result<PathBuf> {
        // 1. Boot VM with Python/Node.js
        let vm = FirecrackerVM::new(VmConfig {
            runtime,
            memory_mb: 128,
            vcpus: 1,
        })?;
        vm.start()?;
        
        // 2. Wait for boot complete
        vm.wait_for_ready()?;
        
        // 3. Create snapshot
        let snapshot_path = format!("/var/nanolambda/snapshots/{}.snap", runtime);
        vm.create_snapshot(&snapshot_path)?;
        
        Ok(snapshot_path.into())
    }
    
    pub fn restore_from_snapshot(&self, runtime: Runtime) -> Result<FirecrackerVM> {
        let snapshot_path = self.base_snapshots.get(&runtime)
            .ok_or("No base snapshot")?;
        
        // Fast restore: 5-10ms instead of 125ms
        FirecrackerVM::restore_from_snapshot(snapshot_path)
    }
}
```

**Success Criteria:**
- [ ] Can create snapshots
- [ ] Can restore from snapshots
- [ ] Restore takes <10ms
- [ ] Multiple VMs from same snapshot

---

#### **Week 8-9: VM Pool Management**

**Tasks:**
1. Pre-warm VM pool
2. Implement VM reuse strategy
3. Add health checks and recycling

```rust
// crates/runtime-microvm/src/pool.rs
pub struct VmPool {
    idle_vms: Vec<FirecrackerVM>,
    active_vms: HashMap<String, FirecrackerVM>,
    config: PoolConfig,
    snapshot_mgr: SnapshotManager,
}

impl VmPool {
    pub fn new(config: PoolConfig) -> Result<Self> {
        let mut pool = Self {
            idle_vms: Vec::new(),
            active_vms: HashMap::new(),
            config,
            snapshot_mgr: SnapshotManager::new()?,
        };
        
        // Pre-warm pool
        pool.prewarm()?;
        
        Ok(pool)
    }
    
    fn prewarm(&mut self) -> Result<()> {
        info!("Pre-warming {} VMs", self.config.min_pool_size);
        
        for _ in 0..self.config.min_pool_size {
            let vm = self.snapshot_mgr.restore_from_snapshot(Runtime::Python)?;
            self.idle_vms.push(vm);
        }
        
        Ok(())
    }
    
    pub async fn get_or_create(&mut self, function_id: &str) -> Result<&mut FirecrackerVM> {
        // Try to get idle VM
        if let Some(vm) = self.idle_vms.pop() {
            self.active_vms.insert(function_id.to_string(), vm);
            return Ok(self.active_vms.get_mut(function_id).unwrap());
        }
        
        // Create new VM from snapshot
        let vm = self.snapshot_mgr.restore_from_snapshot(Runtime::Python)?;
        self.active_vms.insert(function_id.to_string(), vm);
        Ok(self.active_vms.get_mut(function_id).unwrap())
    }
    
    pub fn release(&mut self, function_id: &str) {
        if let Some(vm) = self.active_vms.remove(function_id) {
            // Check if VM is healthy
            if vm.health_check().is_ok() {
                // Reuse for next request
                self.idle_vms.push(vm);
            } else {
                // Discard unhealthy VM
                drop(vm);
            }
        }
    }
}
```

**Success Criteria:**
- [ ] Pool maintains min idle VMs
- [ ] Fast VM allocation (<10ms)
- [ ] Automatic VM recycling
- [ ] Health checks work

---

### **Phase 4: Integration (Weeks 10-12)**

#### **Week 10: API Server Integration**

**Tasks:**
1. Add runtime selection to API server
2. Implement fallback logic
3. Add configuration options

```rust
// crates/api-server/src/executor.rs
pub enum ExecutorBackend {
    Process(ProcessExecutor),
    MicroVM(MicroVMExecutor),
}

pub struct RuntimeManager {
    backend: ExecutorBackend,
}

impl RuntimeManager {
    pub fn new(config: &Config) -> Result<Self> {
        let backend = match config.runtime_type.as_str() {
            "microvm" => {
                info!("Using MicroVM backend");
                ExecutorBackend::MicroVM(MicroVMExecutor::new(config.microvm)?)
            }
            "process" | _ => {
                info!("Using Process backend");
                ExecutorBackend::Process(ProcessExecutor::new(config.process)?)
            }
        };
        
        Ok(Self { backend })
    }
    
    pub async fn execute(
        &mut self,
        function: &Function,
        event: Value,
    ) -> Result<ExecutionResult> {
        match &mut self.backend {
            ExecutorBackend::Process(exec) => exec.execute(&function.code, event).await,
            ExecutorBackend::MicroVM(exec) => exec.execute(&function.code, event).await,
        }
    }
}
```

**Configuration:**
```yaml
# config.yaml
runtime:
  # Choose backend: "process" or "microvm"
  type: "microvm"
  
  # MicroVM-specific settings
  microvm:
    firecracker_path: "/usr/bin/firecracker"
    kernel_path: "/opt/nanolambda/vmlinux"
    rootfs_path: "/opt/nanolambda/rootfs.ext4"
    
    # Pool configuration
    min_pool_size: 5
    max_pool_size: 100
    
    # VM resources
    default_memory_mb: 128
    default_vcpus: 1
    
    # Networking
    enable_networking: false
    
  # Process-specific settings (existing)
  process:
    enable_warm_pool: true
    max_pool_size: 10
```

**Success Criteria:**
- [ ] Can switch between backends via config
- [ ] Both backends work correctly
- [ ] Graceful fallback on failure
- [ ] Same API for both backends

---

#### **Week 11-12: Security Hardening**

**Tasks:**
1. Implement seccomp filters
2. Add network isolation
3. Resource limits via cgroups
4. Audit and penetration testing

```rust
// crates/runtime-microvm/src/security.rs
pub fn apply_security_policy(vm: &FirecrackerVM) -> Result<()> {
    // 1. Enable seccomp filter
    let seccomp_filter = create_seccomp_filter()?;
    vm.apply_seccomp(&seccomp_filter)?;
    
    // 2. Disable networking by default
    vm.configure_network(NetworkPolicy::Disabled)?;
    
    // 3. Apply cgroup limits
    vm.set_cpu_limit(100)?;  // 100% of 1 vCPU
    vm.set_memory_limit_mb(128)?;
    vm.set_pids_max(100)?;
    
    // 4. Drop capabilities
    vm.drop_capabilities(&[
        "CAP_SYS_ADMIN",
        "CAP_NET_ADMIN",
        // ... etc
    ])?;
    
    Ok(())
}

fn create_seccomp_filter() -> Result<SeccompFilter> {
    // Whitelist only essential syscalls
    SeccompFilter::new()
        .allow(&[
            "read", "write", "exit", "rt_sigreturn",
            "open", "close", "stat", "fstat",
            "mmap", "munmap", "brk",
            "execve", "getpid", "getuid",
        ])
        .deny_all_others()
}
```

**Success Criteria:**
- [ ] Seccomp filter blocks dangerous syscalls
- [ ] Network isolated by default
- [ ] Resource limits enforced
- [ ] Security audit passes

---

### **Phase 5: Production Readiness (Weeks 13-16)**

#### **Week 13-14: Testing & Benchmarking**

**Test Suite:**
```bash
# Performance tests
cargo bench --package nanolambda-runtime-microvm

# Security tests
cargo test --package nanolambda-runtime-microvm security

# Integration tests
cargo test --workspace --all-features

# Stress tests
./test-suite/stress-test.sh --runtime microvm --concurrency 1000
```

**Benchmark Targets:**
- [ ] Cold start: <50ms (with snapshot restore)
- [ ] Warm start: <5ms
- [ ] Memory overhead: <128MB per VM
- [ ] Throughput: 1000 req/sec per node

---

#### **Week 15-16: Documentation & Deployment**

**Deliverables:**
1. Installation guide
2. Configuration reference
3. Migration guide (process → microvm)
4. Troubleshooting guide
5. Performance tuning guide

**Example Deployment:**
```bash
# Install dependencies
sudo apt-get install firecracker

# Build with microvm support
cargo build --release --features microvm

# Deploy
./deploy-prod.sh --runtime microvm

# Verify
curl http://localhost:8080/health
# Response: {"runtime": "microvm", "status": "healthy"}
```

**Success Criteria:**
- [ ] Complete documentation
- [ ] Production deployment successful
- [ ] Monitoring and alerting set up
- [ ] Performance meets targets

---

## 📊 **Feature Comparison: Process vs MicroVM**

| Feature | Process Pool | MicroVM |
|---------|-------------|---------|
| **Cold Start** | 12-20ms | 25-50ms (with snapshot) |
| **Warm Start** | 3-5ms | 3-5ms |
| **Memory/Instance** | 42-44MB | 128MB |
| **Isolation** | Good (OS processes) | Excellent (hardware) |
| **Multi-Tenant Safe** | No (shared kernel) | Yes (isolated kernel) |
| **Security** | Good | Excellent |
| **Complexity** | Low | High |
| **Debugging** | Easy | Moderate |
| **Use Cases** | Single-tenant, trusted code | Multi-tenant, untrusted code |

---

## 🚀 **Migration Strategy**

### **Phase 1: Parallel Operation**
```yaml
# Both backends available
runtime:
  type: "process"  # Default stays process

# Users can opt-in to microvm
functions:
  - name: secure-function
    runtime: "microvm"  # Opt-in per function
  - name: regular-function
    runtime: "process"  # Default
```

### **Phase 2: Gradual Migration**
- Week 1-2: Beta users test microvm backend
- Week 3-4: 10% of traffic routed to microvm
- Week 5-6: 50% of traffic
- Week 7-8: 100% migration complete

### **Phase 3: Deprecation**
- Process backend remains available for self-hosted
- Cloud offering uses microvm exclusively

---

## 💰 **Cost-Benefit Analysis**

### **Development Cost:**
- **Team Size:** 2-3 engineers
- **Timeline:** 12-16 weeks
- **Estimated Cost:** $150k - $200k (salary + infrastructure)

### **Benefits:**
- ✅ **Enterprise Sales:** Can compete for regulated industries
- ✅ **Security Positioning:** Match AWS Lambda security model
- ✅ **Higher Pricing:** 2-3x premium for secure tier
- ✅ **Market Expansion:** Banks, healthcare, government

### **Break-Even:**
- Need 10-15 enterprise customers at $2k/month
- Payback period: 6-12 months

---

## ✅ **Success Criteria**

### **Technical:**
- [ ] Cold start <50ms
- [ ] Warm start <5ms
- [ ] 1000+ concurrent VMs per node
- [ ] Zero cross-tenant data leaks
- [ ] Pass security audit

### **Business:**
- [ ] 5+ paying enterprise customers
- [ ] SOC2 compliance achieved
- [ ] Featured in security comparison articles
- [ ] Positive user feedback (NPS >50)

---

## 📚 **Dependencies & Requirements**

### **Software:**
- Firecracker 1.0+
- Linux kernel 5.10+ with KVM
- Rust 1.70+
- QEMU (for testing)

### **Hardware:**
- Intel VT-x or AMD-V
- 8GB RAM minimum (16GB recommended)
- SSD storage (snapshots)

### **Cloud:**
- AWS: i3.metal, c5.metal
- GCP: c2-standard-60 (nested virtualization)
- Bare metal: Any KVM-capable server

---

## 🎯 **Final Recommendation**

**Architecture: Separate Crate in Monorepo** ✅

```
Justification:
├─ Shared infrastructure (API, storage, scheduler)
├─ Optional feature (compile-time flag)
├─ Clean separation of concerns
├─ Easier to maintain than separate project
└─ Users get choice of backend
```

**Timeline: 12-16 weeks** ⏱️

**Go/No-Go Decision Factors:**
- ✅ Have 2-3 engineers available full-time
- ✅ Enterprise customers requesting this feature
- ✅ Budget for $150k-$200k development cost
- ✅ Access to KVM-capable infrastructure
- ✅ Security compliance is competitive requirement

**If NOT all factors met:** Stick with process pool and revisit in 6 months.

---

**Next Step:** Create `crates/runtime-microvm/` and start Week 1 tasks! 🚀
