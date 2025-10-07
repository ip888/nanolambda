# Technical Architecture: NanoLambda Platform

**Date:** October 6, 2025  
**Version:** 1.0  
**Author:** System Architect

---

## 🏗️ System Architecture Overview

NanoLambda is designed as a distributed serverless platform with three main layers:

```
┌────────────────────────────────────────────────────────────┐
│                    Control Plane                           │
│  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐    │
│  │  API Server  │  │  Scheduler   │  │  Registry    │    │
│  │  (actix-web) │  │  (tokio)     │  │  (sled)      │    │
│  └──────────────┘  └──────────────┘  └──────────────┘    │
└────────────────────────────────────────────────────────────┘
                          ↓ gRPC/Unix Sockets
┌────────────────────────────────────────────────────────────┐
│                    Data Plane                              │
│  ┌──────────────────────────────────────────────────────┐ │
│  │          MicroVM Manager (VMM)                       │ │
│  │  ┌─────────┐  ┌─────────┐  ┌─────────┐             │ │
│  │  │ MicroVM │  │ MicroVM │  │ MicroVM │  ... × 1000 │ │
│  │  │  Pool   │  │  Pool   │  │  Pool   │             │ │
│  │  └─────────┘  └─────────┘  └─────────┘             │ │
│  └──────────────────────────────────────────────────────┘ │
└────────────────────────────────────────────────────────────┘
                          ↓ KVM ioctls
┌────────────────────────────────────────────────────────────┐
│                  Infrastructure Layer                      │
│  • Linux Kernel 5.10+ with KVM                            │
│  • x86_64 CPU with VT-x/AMD-V                             │
│  • 16GB+ RAM                                               │
│  • NVMe storage                                            │
└────────────────────────────────────────────────────────────┘
```

---

## 📦 Component Architecture

### 1. API Server (Control Plane)

**Technology:** Rust + actix-web (async HTTP framework)

**Responsibilities:**
- Expose AWS Lambda-compatible REST API
- Handle authentication and authorization
- Function CRUD operations
- Request routing to VMM
- Metrics collection

**Key Endpoints:**

```rust
POST   /2015-03-31/functions              // CreateFunction
GET    /2015-03-31/functions/{name}       // GetFunction
PUT    /2015-03-31/functions/{name}/code  // UpdateFunctionCode
DELETE /2015-03-31/functions/{name}       // DeleteFunction
POST   /2015-03-31/functions/{name}/invocations  // Invoke
GET    /2015-03-31/event-source-mappings  // ListEventSourceMappings
```

**Data Model:**

```rust
pub struct Function {
    pub name: String,
    pub runtime: Runtime,  // Python3.11, NodeJs20, Java21
    pub handler: String,   // "index.handler"
    pub memory_mb: u32,    // 128-10240
    pub timeout_sec: u32,  // 1-900
    pub code_sha256: String,
    pub environment: HashMap<String, String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

pub enum Runtime {
    Python311,
    NodeJs20,
    Java21,
}
```

---

### 2. Scheduler & Orchestrator

**Technology:** Rust + tokio (async runtime)

**Responsibilities:**
- Decide which node/VM to execute function on
- Manage pre-warmed VM pools
- Predictive pre-warming (ML-based)
- Load balancing across nodes
- Resource quota enforcement

**Scheduling Algorithm:**

```rust
pub struct Scheduler {
    predictor: ColdStartPredictor,
    pools: HashMap<FunctionId, VmPool>,
    metrics: MetricsCollector,
}

impl Scheduler {
    pub async fn schedule(&self, request: InvokeRequest) -> Result<VmHandle> {
        // 1. Check if warm VM exists for this function
        if let Some(vm) = self.pools.get(&request.function_id)?.get_warm() {
            return Ok(vm);
        }
        
        // 2. Check if we predicted this invocation (pre-warmed)
        if let Some(vm) = self.predictor.check_prewarmed(&request.function_id) {
            return Ok(vm);
        }
        
        // 3. Cold start: create new VM from snapshot
        let vm = self.create_vm_from_snapshot(&request.function_id).await?;
        
        Ok(vm)
    }
}
```

**Pre-warming Strategy:**

```rust
pub struct ColdStartPredictor {
    // Time-series model (LightGBM or simple moving average for MVP)
    invocation_history: TimeSeries,
}

impl ColdStartPredictor {
    // Predict which functions will be called in next 5 minutes
    pub fn predict_upcoming(&self) -> Vec<FunctionId> {
        // Analyze patterns:
        // - Time of day (e.g., peak at 9am-5pm)
        // - Day of week (weekday vs weekend)
        // - Recent trends (increasing/decreasing)
        // - Event correlations (function A → function B)
        
        // MVP: Simple heuristic
        // Pre-warm if invoked in last 10 minutes
        self.invocation_history
            .last_10_minutes()
            .unique_functions()
    }
}
```

---

### 3. MicroVM Manager (VMM) - Core Engine

**Technology:** Rust + kvm-ioctls + vm-memory

**Responsibilities:**
- Create/destroy microVMs
- Load kernel and initramfs
- Configure vCPUs and memory
- Attach virtio devices (network, block, vsock)
- Snapshot/restore VMs
- Execute functions inside VMs

**VM Lifecycle:**

```rust
pub struct VmManager {
    kvm: Kvm,
    vms: HashMap<VmId, MicroVm>,
    snapshots: SnapshotStore,
}

pub struct MicroVm {
    id: VmId,
    vm_fd: VmFd,
    vcpu_fds: Vec<VcpuFd>,
    memory: GuestMemoryMmap,
    state: VmState,
}

pub enum VmState {
    Created,      // VM created, not started
    Running,      // VM is executing
    Paused,       // VM paused (for snapshot)
    Stopped,      // VM stopped gracefully
}

impl VmManager {
    // Create VM from scratch
    pub fn create_vm(&mut self, config: VmConfig) -> Result<VmId> {
        let vm_fd = self.kvm.create_vm()?;
        
        // Allocate guest memory
        let memory = GuestMemoryMmap::from_ranges(&[
            (GuestAddress(0), config.memory_mb * 1024 * 1024)
        ])?;
        
        // Configure vCPUs
        let vcpu_fds = (0..config.vcpu_count)
            .map(|_| vm_fd.create_vcpu(0))
            .collect::<Result<Vec<_>>>()?;
        
        // Load kernel
        let mut kernel_file = File::open(&config.kernel_path)?;
        let kernel_load_addr = GuestAddress(0x1000000);
        kernel::load_kernel(&memory, &mut kernel_file, kernel_load_addr)?;
        
        // Create microVM struct
        let vm_id = VmId::new();
        let vm = MicroVm {
            id: vm_id,
            vm_fd,
            vcpu_fds,
            memory,
            state: VmState::Created,
        };
        
        self.vms.insert(vm_id, vm);
        Ok(vm_id)
    }
    
    // Restore VM from snapshot (FAST PATH)
    pub fn restore_from_snapshot(&mut self, snapshot_id: &str) -> Result<VmId> {
        let snapshot = self.snapshots.load(snapshot_id)?;
        
        // Create VM
        let vm_id = self.create_vm(snapshot.config)?;
        let vm = self.vms.get_mut(&vm_id)?;
        
        // Restore memory
        snapshot.memory.restore_into(&vm.memory)?;
        
        // Restore vCPU state
        for (vcpu, state) in vm.vcpu_fds.iter().zip(snapshot.vcpu_states.iter()) {
            vcpu.set_regs(&state.regs)?;
            vcpu.set_sregs(&state.sregs)?;
        }
        
        vm.state = VmState::Running;
        Ok(vm_id)
    }
    
    // Execute function in VM
    pub async fn execute_function(
        &mut self,
        vm_id: VmId,
        function: &Function,
        event: serde_json::Value,
    ) -> Result<FunctionResult> {
        let vm = self.vms.get_mut(&vm_id)?;
        
        // Write event to VM via vsock
        vm.send_event(event).await?;
        
        // Wait for result (with timeout)
        let result = timeout(
            Duration::from_secs(function.timeout_sec as u64),
            vm.recv_result()
        ).await??;
        
        Ok(result)
    }
}
```

---

### 4. Runtime Environments

Each language runtime is packaged as a minimal root filesystem.

#### Python Runtime

**Base Image:** ~30MB (Alpine Linux + Python 3.11)

**Structure:**
```
/
├── bin/
│   └── python3.11
├── lib/
│   └── python3.11/
│       └── ... (standard library)
├── usr/
│   └── local/
│       └── lib/
│           └── python3.11/
│               └── site-packages/  (boto3, requests, etc.)
└── function/
    ├── handler.py  (user's function)
    └── bootstrap   (runtime bootstrap)
```

**Bootstrap Process:**

```python
#!/usr/bin/env python3.11
# /function/bootstrap

import os
import sys
import json
import importlib.util

# Read function configuration from environment
handler_path = os.environ['HANDLER']  # "index.handler"
module_name, handler_name = handler_path.rsplit('.', 1)

# Import user's handler
spec = importlib.util.spec_from_file_location(module_name, f"/function/{module_name}.py")
module = importlib.util.module_from_spec(spec)
spec.loader.exec_module(module)
handler = getattr(module, handler_name)

# Event loop: read from vsock, execute, write result
while True:
    # Read event from host via vsock
    event_json = sys.stdin.readline()
    event = json.loads(event_json)
    
    try:
        # Execute user's handler
        result = handler(event, {})
        
        # Write result back to host
        print(json.dumps({"success": True, "result": result}))
    except Exception as e:
        print(json.dumps({"success": False, "error": str(e)}))
    
    sys.stdout.flush()
```

#### Node.js Runtime

**Base Image:** ~40MB (Alpine Linux + Node.js 20)

**Bootstrap (JavaScript):**

```javascript
// /function/bootstrap.js
const fs = require('fs');
const readline = require('readline');

// Load user's handler
const handlerPath = process.env.HANDLER || 'index.handler';
const [moduleName, handlerName] = handlerPath.split('.');
const userModule = require(`/function/${moduleName}`);
const handler = userModule[handlerName];

// Event loop
const rl = readline.createInterface({
  input: process.stdin,
  output: process.stdout,
  terminal: false
});

rl.on('line', async (line) => {
  const event = JSON.parse(line);
  
  try {
    const result = await handler(event, {});
    console.log(JSON.stringify({ success: true, result }));
  } catch (error) {
    console.log(JSON.stringify({ success: false, error: error.message }));
  }
});
```

#### Java Runtime

**Base Image:** ~80MB (Alpine Linux + OpenJDK 21 with JLink)

**Challenges:**
- JVM startup: 1-2 seconds (slow)
- Memory overhead: 50-100MB

**Optimization:**
- Use JLink to create minimal JRE (only needed modules)
- Use GraalVM Native Image for instant startup (future)
- Pre-warm JVM in snapshot

---

### 5. Storage Layer

**Technology:** Sled (embedded database) for MVP, PostgreSQL for production

**Responsibilities:**
- Store function metadata
- Store function code (ZIP files)
- Store invocation logs
- Store snapshots

**Schema:**

```rust
pub struct FunctionRegistry {
    db: sled::Db,
}

impl FunctionRegistry {
    pub fn save_function(&self, function: &Function) -> Result<()> {
        let key = format!("function:{}", function.name);
        let value = serde_json::to_vec(function)?;
        self.db.insert(key, value)?;
        Ok(())
    }
    
    pub fn get_function(&self, name: &str) -> Result<Option<Function>> {
        let key = format!("function:{}", name);
        let value = self.db.get(key)?;
        match value {
            Some(bytes) => Ok(Some(serde_json::from_slice(&bytes)?)),
            None => Ok(None),
        }
    }
    
    pub fn save_code(&self, name: &str, code: &[u8]) -> Result<String> {
        // Calculate SHA256
        let sha256 = sha256_digest(code);
        
        // Store code blob
        let key = format!("code:{}", sha256);
        self.db.insert(key, code)?;
        
        Ok(sha256)
    }
}
```

---

## 🚀 Performance Optimizations

### 1. Cold Start Optimization

**Target:** <5ms cold start time

**Techniques:**

#### Snapshot-based Boot

```rust
// Pre-create snapshot of runtime with function loaded
pub fn create_snapshot(function: &Function) -> Result<Snapshot> {
    // 1. Create VM
    let vm_id = vmm.create_vm(VmConfig {
        memory_mb: function.memory_mb,
        vcpu_count: 1,
        kernel_path: "/snapshots/kernel-5.10",
        rootfs_path: &format!("/snapshots/rootfs-{}", function.runtime),
    })?;
    
    // 2. Start VM and load function
    vmm.start_vm(vm_id)?;
    vmm.load_function(vm_id, function)?;
    
    // 3. Wait for VM to be ready (runtime initialized)
    vmm.wait_for_ready(vm_id, Duration::from_secs(5))?;
    
    // 4. Pause VM and snapshot
    vmm.pause_vm(vm_id)?;
    let snapshot = vmm.snapshot_vm(vm_id)?;
    
    // 5. Cleanup
    vmm.destroy_vm(vm_id)?;
    
    Ok(snapshot)
}

// Restore from snapshot (fast)
pub fn restore_and_invoke(snapshot_id: &str, event: Value) -> Result<Value> {
    // Restore VM from snapshot (2-3ms)
    let vm_id = vmm.restore_from_snapshot(snapshot_id)?;
    
    // Resume VM (1ms)
    vmm.resume_vm(vm_id)?;
    
    // Execute function (1ms to send event, then depends on function)
    let result = vmm.execute_function(vm_id, event)?;
    
    Ok(result)
}
```

#### Memory Deduplication

```rust
// Use KSM (Kernel Same-page Merging) for identical pages
// Multiple VMs running same runtime share memory pages

pub fn enable_ksm_for_vm(vm: &MicroVm) -> Result<()> {
    // Mark VM memory as mergeable
    for region in vm.memory.regions() {
        unsafe {
            libc::madvise(
                region.as_ptr() as *mut libc::c_void,
                region.len(),
                libc::MADV_MERGEABLE
            );
        }
    }
    Ok(())
}
```

### 2. Network Performance

**Challenge:** virtio-net overhead

**Solution:** Use vhost-net for kernel-level packet processing

```rust
pub fn setup_vhost_net(vm: &MicroVm) -> Result<()> {
    let vhost_fd = fs::OpenOptions::new()
        .read(true)
        .write(true)
        .open("/dev/vhost-net")?;
    
    // Configure vhost-net backend
    // Packets bypass userspace, directly to VM
    vm.configure_vhost(vhost_fd)?;
    
    Ok(())
}
```

### 3. Concurrency Model

**Challenge:** Handle 1000s of concurrent VMs

**Solution:** Async I/O with tokio

```rust
#[tokio::main]
async fn main() {
    let vmm = Arc::new(Mutex::new(VmManager::new()));
    
    // Spawn tasks for each invocation
    let handles: Vec<_> = invocations
        .into_iter()
        .map(|inv| {
            let vmm = vmm.clone();
            tokio::spawn(async move {
                vmm.lock().await.invoke(inv).await
            })
        })
        .collect();
    
    // Wait for all to complete
    for handle in handles {
        handle.await?;
    }
}
```

---

## 🔒 Security Architecture

### Multi-Tenant Isolation

**Layers of Isolation:**

1. **Hardware:** Each function runs in separate microVM (KVM)
2. **Network:** Private network namespace per VM
3. **Filesystem:** Separate rootfs per VM
4. **Memory:** No shared memory between VMs
5. **Seccomp:** Syscall filtering

**Security Model:**

```rust
pub fn create_isolated_vm(config: VmConfig) -> Result<MicroVm> {
    // 1. Create VM with minimal privileges
    let vm = create_vm(config)?;
    
    // 2. Apply seccomp filter (whitelist syscalls)
    apply_seccomp_filter(&vm, &ALLOWED_SYSCALLS)?;
    
    // 3. Set resource limits (cgroups)
    set_cgroup_limits(&vm, CgroupLimits {
        cpu_quota: config.cpu_percent,
        memory_limit: config.memory_mb * 1024 * 1024,
        pids_max: 1024,
    })?;
    
    // 4. Network isolation (no internet by default)
    configure_network_namespace(&vm, NetworkPolicy::Isolated)?;
    
    Ok(vm)
}

const ALLOWED_SYSCALLS: &[i32] = &[
    libc::SYS_read,
    libc::SYS_write,
    libc::SYS_exit,
    libc::SYS_rt_sigreturn,
    // ... minimal set
];
```

---

## 📊 Monitoring & Observability

### Metrics Collection

```rust
use prometheus::{Counter, Histogram, Registry};

pub struct Metrics {
    invocations_total: Counter,
    cold_starts_total: Counter,
    execution_duration: Histogram,
    memory_usage: Histogram,
}

impl Metrics {
    pub fn record_invocation(&self, function: &str, duration: Duration, cold_start: bool) {
        self.invocations_total
            .with_label_values(&[function])
            .inc();
        
        if cold_start {
            self.cold_starts_total
                .with_label_values(&[function])
                .inc();
        }
        
        self.execution_duration
            .with_label_values(&[function])
            .observe(duration.as_secs_f64());
    }
}
```

### Distributed Tracing

```rust
use tracing::{info, span, Level};

#[tracing::instrument]
pub async fn invoke_function(request: InvokeRequest) -> Result<InvokeResponse> {
    let span = span!(Level::INFO, "invoke", function = %request.function_name);
    let _enter = span.enter();
    
    info!("Starting invocation");
    
    let vm = scheduler.schedule(&request).await?;
    let result = vm.execute(request.event).await?;
    
    info!("Invocation complete");
    Ok(result)
}
```

---

## 🌐 Deployment Architecture

### Single-Node Deployment (MVP)

```yaml
# docker-compose.yml
version: '3.8'

services:
  nanolambda:
    image: nanolambda:latest
    privileged: true  # Required for KVM
    devices:
      - /dev/kvm:/dev/kvm
    volumes:
      - ./functions:/functions
      - ./snapshots:/snapshots
    ports:
      - "8080:8080"  # API
      - "9090:9090"  # Metrics
    environment:
      - RUST_LOG=info
```

### Multi-Node Deployment (Production)

```yaml
# kubernetes/deployment.yaml
apiVersion: apps/v1
kind: DaemonSet
metadata:
  name: nanolambda-worker
spec:
  selector:
    matchLabels:
      app: nanolambda-worker
  template:
    spec:
      hostNetwork: true  # For KVM access
      containers:
      - name: worker
        image: nanolambda-worker:latest
        securityContext:
          privileged: true
        volumeMounts:
        - name: dev-kvm
          mountPath: /dev/kvm
      volumes:
      - name: dev-kvm
        hostPath:
          path: /dev/kvm
```

---

## 📚 Technology Stack Summary

| Component | Technology | Rationale |
|-----------|-----------|-----------|
| **Core VMM** | Rust + kvm-ioctls | Memory safety, performance |
| **API Server** | Rust + actix-web | High-performance async HTTP |
| **Storage** | Sled (MVP) → PostgreSQL | Embedded → Distributed |
| **Messaging** | Unix sockets / gRPC | Low latency IPC |
| **Monitoring** | Prometheus + Grafana | Industry standard |
| **Tracing** | OpenTelemetry | Distributed tracing |
| **Deployment** | Docker + Kubernetes | Container orchestration |

---

**Document Version:** 1.0  
**Last Updated:** October 6, 2025  
**Next Review:** November 6, 2025
