# Recommendation: tech stack and architecture

### Language choice
- **Primary:** **Rust** for your VMM, device backends, and control plane agents. You get memory safety for attack-prone virtual device code without sacrificing speed; this is exactly why Firecracker/crosvm/clh chose it.
- **C (selectively):** only when you must talk to low-level kernel bits (headers, ioctls you don’t want to rewrap), or to reuse mature C libs. If you write new C, stick to **C11/C17** for portability across distros/kernels and CI toolchains.
- **Interoperability:** bind via **FFI** from Rust; or prefer existing **rust-vmm** crates (kvm-ioctls, vm-memory, vmm-sys-util, linux-loader, virtio components) to avoid writing either language yourself.

### Building blocks (proven & reusable)
- **VMM layer:** base on **rust-vmm**; target **KVM** first, optionally add **MSHV** later (Windows hosts), as **Cloud Hypervisor** already does.
- **Paravirtual devices:** virtio-net, virtio-blk, vsock; consider **vhost-user** daemons (there’s an active Rust ecosystem for device daemons).
- **Guest strategy:**  
  1) **Minimal Linux guest** (Alpine/nanos/initramfs) booting your function, or  
  2) **Unikernel** for Rust workloads (e.g., **Hermit**), or  
  3) **WASM in a guest** (Wasmtime/WasmEdge) for ultra-fast cold starts with strong isolation.

---

# Where you can compete (niches with room for a SaaS)

1) **Confidential-by-default serverless (SNP/TDX) with micro-VMs**  
   Deliver: per-request attestation, key release, and signed execution reports in <250ms cold starts.

2) **Cross-hypervisor, cross-OS function isolation**  
   A managed runtime that runs on **Linux/KVM** *and* **Windows/MSHV** broadens install-base.

3) **GPU-friendly micro-VM functions**  
   Fast attach/detach of vGPU/SR-IOV slices for short-lived inference jobs (LLM, vision).

4) **Edge-optimized functions on ARM64**  
   Uniform stack for **arm64** edge nodes (retail/5G/industrial) with WASM or Rust unikernels.

5) **“Container-feel” developer experience over micro-VMs**  
   Drop-in shim for Tekton/GitHub Actions self-hosted runners.

---

# Competitive landscape at a glance

- **Firecracker (AWS):** Rust, KVM, minimal devices, powers Lambda/Fargate.  
- **Cloud Hypervisor (Intel & community):** Rust VMM for modern cloud workloads; supports **KVM and MSHV**.  
- **crosvm (Google):** Rust VMM used for ChromeOS/Android virtualization.  
- **QEMU microvm (C):** microvm machine type mimics Firecracker model but within QEMU.  
- **gVisor (Go):** user-space kernel sandbox, not a VM.  
- **Wasm runtimes (WasmEdge/Wasmtime):** cold-start speeds & portability.  
- **ACRN (Intel):** embedded/real-time hypervisor (C).

---

# A concrete stack you can build (and sell)

**Host node (Linux, later Windows):**
- **VMM:** Rust daemon built on **rust-vmm**  
- **Devices:** virtio-net/blk/vsock; add **vhost-user** daemons for fs/gpu/video  
- **Guest options:** minimal Linux, Hermit unikernel, or Wasmtime/WasmEdge  
- **Security:** seccomp, namespaces, SNP/TDX  
- **Networking:** eBPF shaping, WireGuard overlays

**Control plane (multi-tenant SaaS):**
- API & scheduler, image builder, secrets broker, billing, CRI plugin

**Pricing:** per-request + GB-s + vCPU-s + premium features (confidential, GPU, edge)

---

# Development plan (12–16 week path to paid design partners)

1) **MVP (4–6 weeks):** Boot minimal Linux guest in <150ms, virtio devices, CLI `fns run`  
2) **DX Layer (2–3 weeks):** Docker-like UX, GitHub Actions integration  
3) **Differentiator track:** Confidential FaaS / GPU micro-functions / Windows MSHV  
4) **Private beta:** 3–5 design partners

---

# Bottom line

- **Use Rust as your default**, keep C minimal (C11/C17).  
- **Pick one sharp niche** (confidential, GPU, or cross-hypervisor).  
- **Your moat is operational simplicity + DX**, not raw hypervisor code.

---

# What to Focus on from Day One

### 1. Cold Start + Security
- Snapshot/restore <100ms
- Confidential compute attestation
- Strong tenant isolation

### 2. Developer Experience
- Docker/K8s-like CLI + API
- One command to run a function with isolation & proof

### 3. Vertical Differentiators
- **Confidential FaaS** (banks, healthcare)  
- **GPU Micro-Functions** (AI workloads)  
- **Edge FaaS on ARM64** (IoT, telco)

---

# Value to Customers vs Investors

| Idea | Customer Value | Investor Appeal | Competition |
|------|----------------|-----------------|-------------|
| Confidential FaaS | Trust, compliance | High | Few players |
| GPU Micro-Functions | Cost savings, hot AI | Very High | Cloud GPU bottleneck |
| Edge ARM64 FaaS | Reliable edge | Moderate | IoT platforms |
| Generic MicroVM FaaS | Cheap compute | Low | AWS, GCP, Azure |

---

# Tech Stack Recommendation

- **Core VMM:** Rust + rust-vmm  
- **Guest:** minimal Linux or Hermit unikernel  
- **Devices:** virtio + vhost-user for GPU  
- **Security:** SEV-SNP/TDX  
- **DX Layer:** CLI + API, K8s CRI plugin  
- **Business Model:** SaaS & Enterprise license

---

# How to Be Competitive from Day One

1. Don’t just build a VMM — package DX  
2. Lead with niche (Confidential/GPU FaaS)  
3. MVP in 3 months: boot, run, prove  
4. Publish OSS + SaaS beta  
