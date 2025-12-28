# Action Plan: Remove Placeholder VMM, Implement Real MicroVM Runtime

## 🎯 **Summary**

Your project currently has a **placeholder VMM crate** that:
- ❌ Doesn't work (KVM permission issues)
- ❌ Isn't used in production
- ❌ Confuses contributors

**Decision:** Remove placeholder, implement proper microVM runtime as new crate.

---

## 📋 **Step-by-Step Action Plan**

### **Step 1: Remove Placeholder VMM Crate** (15 minutes)

```bash
# 1. Remove the placeholder crate
rm -rf crates/vmm/

# 2. Update workspace Cargo.toml
# Remove from members list:
sed -i '/crates\/vmm/d' Cargo.toml

# 3. Remove from lib.rs
sed -i '/pub use nanolambda_vmm/d' src/lib.rs

# 4. Remove POC binary
rm src/bin/nanolambda-poc.rs

# 5. Update docs to remove references
sed -i 's/--exclude nanolambda-vmm//g' docs/*.md

# 6. Commit removal
git add -A
git commit -m "Remove placeholder VMM crate - will be replaced with real implementation"
```

---

### **Step 2: Create New MicroVM Runtime Crate** (30 minutes)

```bash
# Create new crate with proper structure
cargo new --lib crates/runtime-microvm

cd crates/runtime-microvm

# Create directory structure
mkdir -p src/{vmm,guest,snapshots}
mkdir -p guest-images/rootfs
mkdir -p tests

# Add to workspace
cd ../..
echo "    \"crates/runtime-microvm\"," >> Cargo.toml
```

**Cargo.toml for new crate:**
```toml
[package]
name = "nanolambda-runtime-microvm"
version = "0.1.0"
edition = "2021"

[dependencies]
# Core dependencies
tokio = { workspace = true }
serde = { workspace = true }
serde_json = { workspace = true }
anyhow = { workspace = true }
thiserror = { workspace = true }
tracing = { workspace = true }

# Runtime trait (shared interface)
nanolambda-runtime = { path = "../runtime" }

# Firecracker integration
firecracker-sdk = "0.1"  # Or direct binary execution

# Virtualization
kvm-ioctls = "0.16"
kvm-bindings = "0.9"
vm-memory = "0.14"
vmm-sys-util = "0.12"
linux-loader = "0.11"

# Networking
vsock = "0.4"

# Optional features
[features]
default = []
snapshot-support = []
network-isolation = []
```

---

### **Step 3: Implement Core Interface** (Week 1)

**File: `crates/runtime-microvm/src/lib.rs`**

```rust
//! MicroVM-based runtime for secure multi-tenant execution
//! 
//! This crate provides Firecracker-based microVM isolation for running
//! untrusted code in a hardware-isolated environment.

mod executor;
mod firecracker;
mod pool;
mod vmm;
mod guest;
mod snapshots;

pub use executor::MicroVMExecutor;
pub use firecracker::FirecrackerVM;
pub use pool::{VmPool, PoolConfig};

use nanolambda_runtime::RuntimeTrait;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum MicroVMError {
    #[error("VM creation failed: {0}")]
    VmCreationFailed(String),
    
    #[error("Execution failed: {0}")]
    ExecutionFailed(String),
    
    #[error("Communication error: {0}")]
    CommunicationError(String),
    
    #[error("Snapshot error: {0}")]
    SnapshotError(String),
}

pub type Result<T> = std::result::Result<T, MicroVMError>;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MicroVMConfig {
    /// Path to Firecracker binary
    pub firecracker_path: PathBuf,
    
    /// Path to guest kernel
    pub kernel_path: PathBuf,
    
    /// Path to root filesystem
    pub rootfs_path: PathBuf,
    
    /// Default memory per VM (MB)
    pub default_memory_mb: u32,
    
    /// Default vCPUs per VM
    pub default_vcpus: u32,
    
    /// Pool configuration
    pub pool: PoolConfig,
    
    /// Enable networking
    pub enable_networking: bool,
}

impl Default for MicroVMConfig {
    fn default() -> Self {
        Self {
            firecracker_path: PathBuf::from("/usr/bin/firecracker"),
            kernel_path: PathBuf::from("/opt/nanolambda/vmlinux"),
            rootfs_path: PathBuf::from("/opt/nanolambda/rootfs.ext4"),
            default_memory_mb: 128,
            default_vcpus: 1,
            pool: PoolConfig::default(),
            enable_networking: false,
        }
    }
}
```

---

### **Step 4: Update API Server to Support Both Runtimes** (Week 10)

**File: `crates/api-server/Cargo.toml`**

```toml
[dependencies]
# ... existing deps ...

# Runtime backends
nanolambda-runtime = { path = "../runtime" }
nanolambda-runtime-microvm = { path = "../runtime-microvm", optional = true }

[features]
default = ["process-runtime"]
process-runtime = []
microvm-runtime = ["nanolambda-runtime-microvm"]
```

**File: `crates/api-server/src/runtime_manager.rs` (NEW)**

```rust
use nanolambda_runtime::{RuntimeTrait, ExecutionResult};
use serde::Deserialize;
use serde_json::Value;

#[derive(Deserialize)]
pub struct RuntimeConfig {
    #[serde(rename = "type")]
    pub runtime_type: String,
    
    #[serde(default)]
    pub process: ProcessConfig,
    
    #[cfg(feature = "microvm-runtime")]
    #[serde(default)]
    pub microvm: MicroVMConfig,
}

pub enum RuntimeBackend {
    Process(ProcessExecutor),
    
    #[cfg(feature = "microvm-runtime")]
    MicroVM(MicroVMExecutor),
}

pub struct RuntimeManager {
    backend: RuntimeBackend,
}

impl RuntimeManager {
    pub fn new(config: RuntimeConfig) -> Result<Self> {
        let backend = match config.runtime_type.as_str() {
            #[cfg(feature = "microvm-runtime")]
            "microvm" => {
                info!("Initializing MicroVM runtime");
                RuntimeBackend::MicroVM(MicroVMExecutor::new(config.microvm)?)
            }
            
            "process" | _ => {
                info!("Initializing Process runtime");
                RuntimeBackend::Process(ProcessExecutor::new(config.process)?)
            }
        };
        
        Ok(Self { backend })
    }
    
    pub async fn execute(
        &mut self,
        code: &str,
        event: Value,
        config: &FunctionConfig,
    ) -> Result<ExecutionResult> {
        match &mut self.backend {
            RuntimeBackend::Process(exec) => {
                exec.execute(code, event, config).await
            }
            
            #[cfg(feature = "microvm-runtime")]
            RuntimeBackend::MicroVM(exec) => {
                exec.execute(code, event, config).await
            }
        }
    }
}
```

---

### **Step 5: Update Configuration** (Week 10)

**File: `config/default.yaml`**

```yaml
# NanoLambda Configuration

api_server:
  host: "0.0.0.0"
  port: 8080

# Runtime backend selection
runtime:
  # Options: "process" or "microvm"
  type: "process"
  
  # Process-based runtime (default)
  process:
    enable_warm_pool: true
    max_pool_size: 10
    process_idle_timeout_secs: 300
    use_inline_execution: true
  
  # MicroVM-based runtime (optional, requires KVM)
  # Uncomment to use:
  # microvm:
  #   firecracker_path: "/usr/bin/firecracker"
  #   kernel_path: "/opt/nanolambda/vmlinux"
  #   rootfs_path: "/opt/nanolambda/rootfs.ext4"
  #   default_memory_mb: 128
  #   default_vcpus: 1
  #   enable_networking: false
  #   
  #   pool:
  #     min_pool_size: 5
  #     max_pool_size: 100
  #     idle_timeout_secs: 600

storage:
  type: "sqlite"
  sqlite:
    path: "./data/nanolambda.db"
```

---

## 🚀 **Quick Start Commands**

### **Option 1: Use Process Runtime (Current, Default)**

```bash
# Build and run (default process runtime)
cargo build --release
./target/release/nanolambda-server

# Config uses process runtime by default
# No changes needed!
```

### **Option 2: Use MicroVM Runtime (New, Requires KVM)**

```bash
# 1. Install Firecracker
wget https://github.com/firecracker-microvm/firecracker/releases/download/v1.6.0/firecracker-v1.6.0-x86_64.tgz
tar xzf firecracker-v1.6.0-x86_64.tgz
sudo cp release-v1.6.0-x86_64/firecracker-v1.6.0-x86_64 /usr/bin/firecracker
sudo chmod +x /usr/bin/firecracker

# 2. Build guest kernel (see MICROVM_IMPLEMENTATION_PLAN.md)
cd crates/runtime-microvm/guest-images
./build-kernel.sh
./build-rootfs.sh

# 3. Build with microvm feature
cargo build --release --features microvm-runtime

# 4. Update config.yaml
# Change: runtime.type: "microvm"

# 5. Run
./target/release/nanolambda-server
```

---

## 📊 **Migration Timeline**

| Phase | Duration | Description |
|-------|----------|-------------|
| **Phase 0: Cleanup** | 1 day | Remove placeholder VMM crate |
| **Phase 1: Foundation** | 3 weeks | New crate structure, Firecracker integration |
| **Phase 2: Guest Runtime** | 3 weeks | Init system, IPC, Python/Node support |
| **Phase 3: Performance** | 3 weeks | Snapshots, VM pooling |
| **Phase 4: Integration** | 3 weeks | API server, config, fallback logic |
| **Phase 5: Production** | 4 weeks | Testing, docs, deployment |
| **Total** | **16 weeks** | **4 months** |

---

## ✅ **Acceptance Criteria**

### **Technical:**
- [ ] Placeholder VMM removed
- [ ] New `runtime-microvm` crate compiles
- [ ] API server supports both backends
- [ ] Config file allows runtime selection
- [ ] Process runtime still works (default)
- [ ] MicroVM runtime passes security tests
- [ ] Cold start <50ms with snapshots
- [ ] Documentation complete

### **Business:**
- [ ] Zero disruption to existing users
- [ ] Clear migration path documented
- [ ] Enterprise tier pricing defined
- [ ] Marketing materials ready

---

## 🎯 **Recommendation**

**Execute Step 1 NOW:**  Remove the placeholder VMM crate to eliminate confusion.

**Execute Steps 2-5 over next 16 weeks:** Follow the detailed implementation plan in [MICROVM_IMPLEMENTATION_PLAN.md](MICROVM_IMPLEMENTATION_PLAN.md).

**Key Principle:** 
> Process runtime remains default and fully supported.  
> MicroVM runtime is optional, enterprise-grade upgrade.  
> Users choose based on their security requirements.

---

Ready to start? Run these commands:

```bash
# Step 1: Clean up
git rm -rf crates/vmm/
git rm src/bin/nanolambda-poc.rs

# Step 2: Create new structure
cargo new --lib crates/runtime-microvm

# Step 3: Commit
git add -A
git commit -m "Setup: Remove placeholder VMM, create runtime-microvm crate structure"
git push
```

🚀 **Let's build production-ready microVM isolation!**
