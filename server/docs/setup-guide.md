# Setup Guide: NanoLambda Development Environment

**Target:** Get development environment ready for KVM-based microVM development  
**Time Required:** 30-60 minutes  
**Last Updated:** October 6, 2025

---

## 🚨 Prerequisites

### Hardware Requirements

**❌ Will NOT work on:**
- Apple Silicon (M1/M2/M3) Macs
- Windows without WSL2 + nested virtualization
- ARM-based systems (Raspberry Pi, etc.)

**✅ Will work on:**
- Intel/AMD x86_64 CPUs with VT-x/AMD-V
- Linux (Ubuntu 22.04+, Debian 11+, Fedora 36+)
- Cloud VMs (AWS EC2, GCP, Azure, DigitalOcean)

### Why KVM Requirement?

NanoLambda uses **KVM (Kernel Virtual Machine)** for hardware-based isolation. KVM requires:
1. x86_64 CPU with virtualization extensions (VT-x for Intel, AMD-V for AMD)
2. Linux kernel with KVM module
3. `/dev/kvm` device available

---

## 🎯 Recommended Setup Options

### Option A: GitHub Codespaces (EASIEST - Recommended for Mac Users)

**Pros:**
- ✅ Works from any computer (including M1 Mac!)
- ✅ Pre-configured Linux environment
- ✅ No local setup needed
- ✅ VS Code in browser or local
- ✅ Start coding in 5 minutes

**Cons:**
- ⚠️ Costs ~$0.18/hour (~$30/month for 8hrs/day)
- ⚠️ Requires internet connection

**Setup Steps:**

1. **Fork/Clone Repository:**
   ```bash
   # On GitHub.com, click "Fork" or create new repo
   ```

2. **Create Codespace:**
   - Click "Code" button → "Codespaces" → "New codespace"
   - Select machine type: **4-core (8GB RAM)**
   - Wait 2-3 minutes for environment to start

3. **Verify KVM:**
   ```bash
   # In Codespace terminal:
   sudo apt update
   sudo apt install -y qemu-kvm libvirt-daemon-system cpu-checker
   
   # Check if KVM available
   sudo kvm-ok
   
   # Should output:
   # INFO: /dev/kvm exists
   # KVM acceleration can be used
   ```

4. **Install Rust:**
   ```bash
   curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
   source $HOME/.cargo/env
   
   # Verify
   rustc --version
   cargo --version
   ```

5. **Test Build:**
   ```bash
   cd /workspaces/nanolambda  # Or your repo name
   cargo build
   ```

**Done!** You can now develop in VS Code (browser or local via Remote extension).

---

### Option B: AWS EC2 Instance (BEST FOR SERIOUS DEVELOPMENT)

**Pros:**
- ✅ Full control over environment
- ✅ Can leave builds running overnight
- ✅ Scale up for production testing
- ✅ ~$30/month for t3.medium

**Cons:**
- ⚠️ Requires AWS account
- ⚠️ Need to manage instance lifecycle
- ⚠️ SSH/networking setup

**Setup Steps:**

1. **Launch EC2 Instance:**
   - **AMI:** Ubuntu Server 22.04 LTS
   - **Instance Type:** t3.medium (2 vCPU, 4GB RAM) for dev
     - For production testing: t3.large or c6i.xlarge
   - **Storage:** 30GB GP3 SSD
   - **Security Group:** 
     - SSH (22) from your IP
     - Custom TCP (8080) for API testing
   - **Advanced Details → Metadata:** Enable "IMDSv2 required"

2. **Enable Nested Virtualization (Important!):**
   
   After instance launches:
   ```bash
   # SSH into instance
   ssh -i your-key.pem ubuntu@ec2-xx-xx-xx-xx.compute.amazonaws.com
   
   # Check CPU flags
   egrep -o '(vmx|svm)' /proc/cpuinfo | wc -l
   # Should be > 0 (number of CPU cores)
   
   # Install KVM
   sudo apt update
   sudo apt install -y qemu-kvm libvirt-daemon-system bridge-utils cpu-checker
   
   # Add your user to kvm group
   sudo usermod -aG kvm $USER
   sudo usermod -aG libvirt $USER
   
   # Logout and login again
   exit
   ssh -i your-key.pem ubuntu@ec2-xx-xx-xx-xx.compute.amazonaws.com
   
   # Verify
   sudo kvm-ok
   # Should say: KVM acceleration can be used
   ```

3. **Install Rust:**
   ```bash
   curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
   # Select: 1 (default installation)
   
   source $HOME/.cargo/env
   
   # Verify
   rustc --version  # Should be 1.70+
   ```

4. **Install Development Tools:**
   ```bash
   sudo apt install -y build-essential git curl pkg-config libssl-dev
   ```

5. **Clone Repository:**
   ```bash
   git clone https://github.com/yourusername/nanolambda.git
   cd nanolambda
   cargo build
   ```

6. **Set Up VS Code Remote (Optional but Recommended):**
   
   On your local machine:
   - Install VS Code
   - Install "Remote - SSH" extension
   - Connect to EC2 instance:
     ```
     Host nanolambda-dev
         HostName ec2-xx-xx-xx-xx.compute.amazonaws.com
         User ubuntu
         IdentityFile ~/.ssh/your-key.pem
     ```
   - Open folder: `/home/ubuntu/nanolambda`

---

### Option C: Hetzner Dedicated Server (BEST VALUE)

**Pros:**
- ✅ Cheap ($40/month for powerful server)
- ✅ Root access
- ✅ Good for long-term development

**Cons:**
- ⚠️ European data centers (higher latency from US)
- ⚠️ Manual setup

**Setup Steps:**

1. **Order Server:**
   - Go to https://www.hetzner.com/dedicated-rootserver
   - Select: AX41-NVMe (AMD Ryzen 5, 64GB RAM, ~$40/month)
   - OS: Ubuntu 22.04

2. **SSH In & Setup:**
   ```bash
   ssh root@your-server-ip
   
   # Install packages
   apt update
   apt install -y qemu-kvm libvirt-daemon-system build-essential git curl
   
   # Create dev user
   adduser dev
   usermod -aG sudo,kvm,libvirt dev
   
   # Switch to dev user
   su - dev
   
   # Install Rust
   curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
   source $HOME/.cargo/env
   
   # Clone repo
   git clone https://github.com/yourusername/nanolambda.git
   cd nanolambda
   cargo build
   ```

---

### Option D: Local Linux Machine

**If you already have x86_64 Linux:**

```bash
# Install KVM
sudo apt install -y qemu-kvm libvirt-daemon-system cpu-checker

# Check
sudo kvm-ok

# Install Rust
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source $HOME/.cargo/env

# Clone and build
git clone https://github.com/yourusername/nanolambda.git
cd nanolambda
cargo build
```

---

## ✅ Verify Your Setup

### Test 1: KVM Access

```bash
# Check /dev/kvm exists
ls -l /dev/kvm
# Should show: crw-rw---- 1 root kvm ...

# Check you're in kvm group
groups
# Should include: kvm libvirt

# Test KVM creation
sudo kvm-ok
# Should say: KVM acceleration can be used
```

### Test 2: Rust Toolchain

```bash
# Check Rust version
rustc --version
# Should be: rustc 1.70.0 or higher

# Check Cargo
cargo --version

# Test compilation
cargo new test_project
cd test_project
cargo build --release
./target/release/test_project
# Should print: Hello, world!
```

### Test 3: Build NanoLambda

```bash
cd ~/nanolambda  # Or your path
cargo build

# Should compile without errors
# First build takes 5-10 minutes (downloads dependencies)

# Run tests
cargo test
```

---

## 🔧 Development Tools (Optional but Recommended)

### Rust Tooling

```bash
# Rust formatter
rustup component add rustfmt

# Linter
rustup component add clippy

# LSP for IDE
rustup component add rust-analyzer
```

### VS Code Extensions

If using VS Code:
- **rust-analyzer** - Rust language support
- **Better TOML** - TOML file support
- **Error Lens** - Inline error display
- **GitLens** - Git integration

### Useful Tools

```bash
# htop - monitor CPU/memory
sudo apt install htop

# ripgrep - fast code search
cargo install ripgrep

# fd - fast find
cargo install fd-find

# tokei - count lines of code
cargo install tokei
```

---

## 📁 Project Structure

After cloning, your directory should look like:

```
nanolambda/
├── Cargo.toml          # Rust dependencies
├── Cargo.lock          # Locked dependencies
├── README.md           # Project overview
├── LICENSE             # MIT license
├── .gitignore          # Git ignore rules
├── docs/               # Documentation
│   ├── 00-executive-summary.md
│   ├── 01-market-analysis.md
│   ├── 02-technical-architecture.md
│   ├── 04-roadmap.md
│   └── setup-guide.md  # This file
├── src/                # Source code
│   ├── main.rs         # Entry point
│   ├── lib.rs          # Library root
│   ├── api/            # REST API server
│   ├── vmm/            # Virtual machine manager
│   ├── runtime/        # Language runtimes
│   ├── scheduler/      # Orchestration
│   └── storage/        # Function registry
├── tests/              # Integration tests
├── deploy/             # Deployment configs
│   ├── docker/
│   ├── kubernetes/
│   └── systemd/
├── scripts/            # Utility scripts
└── kernels/            # Linux kernels (downloaded separately)
```

---

## 🚀 Next Steps

Once your environment is set up:

1. **Read the Roadmap:**
   ```bash
   cat docs/04-roadmap.md
   ```

2. **Start Week 1 Tasks:**
   - Review KVM documentation
   - Study `kvm-ioctls` crate
   - Begin Day 1-2 tasks (already done - project structure created!)

3. **Join Community (Future):**
   - Discord/Slack (once created)
   - GitHub Discussions

---

## 🐛 Troubleshooting

### Problem: "kvm-ok" says KVM not available

**Solution:**
```bash
# Check CPU supports virtualization
egrep -o '(vmx|svm)' /proc/cpuinfo

# If empty, your CPU doesn't support virtualization
# You need a different machine

# If shows vmx/svm, load KVM module
sudo modprobe kvm_intel  # For Intel
# OR
sudo modprobe kvm_amd    # For AMD

# Make persistent
echo "kvm_intel" | sudo tee -a /etc/modules  # Intel
# OR
echo "kvm_amd" | sudo tee -a /etc/modules    # AMD
```

### Problem: Permission denied on /dev/kvm

**Solution:**
```bash
# Check permissions
ls -l /dev/kvm

# Add yourself to kvm group
sudo usermod -aG kvm $USER

# Log out and back in
exit
# SSH back in

# Verify
groups  # Should include kvm
```

### Problem: Cargo build fails with linking errors

**Solution:**
```bash
# Install build dependencies
sudo apt install -y build-essential pkg-config libssl-dev

# Clean and rebuild
cargo clean
cargo build
```

### Problem: Out of disk space

**Solution:**
```bash
# Check space
df -h

# Clean Cargo cache
cargo clean
rm -rf ~/.cargo/registry/cache

# If on cloud, resize disk (AWS example):
# 1. Resize EBS volume in AWS Console
# 2. Grow partition
sudo growpart /dev/nvme0n1 1
sudo resize2fs /dev/nvme0n1p1
```

---

## 💰 Cost Estimation

### GitHub Codespaces
```
4-core machine: $0.18/hour
8 hours/day × 20 days = $28.80/month
Storage: $0.07/GB × 10GB = $0.70/month
Total: ~$30/month
```

### AWS EC2 (t3.medium)
```
Instance: $0.0416/hour × 730 hours = $30.37/month
Storage: 30GB × $0.08 = $2.40/month
Data transfer: ~$5/month
Total: ~$38/month
```

### Hetzner Dedicated
```
AX41-NVMe: €39/month (~$42/month)
Includes: 64GB RAM, 2×512GB NVMe
Total: $42/month (best value)
```

---

## 📚 Additional Resources

### Rust Learning
- [The Rust Book](https://doc.rust-lang.org/book/)
- [Rust by Example](https://doc.rust-lang.org/rust-by-example/)
- [Rustlings](https://github.com/rust-lang/rustlings) - Interactive exercises

### KVM Resources
- [KVM Documentation](https://www.linux-kvm.org/page/Documents)
- [kvm-ioctls Crate Docs](https://docs.rs/kvm-ioctls/)
- [Firecracker Design Doc](https://github.com/firecracker-microvm/firecracker/blob/main/docs/design.md)

### Virtualization
- [Intel VT-x Specs](https://www.intel.com/content/www/us/en/virtualization/virtualization-technology/intel-virtualization-technology.html)
- [AMD-V Specs](https://www.amd.com/en/technologies/virtualization)

---

## ✅ Setup Complete!

If you've successfully:
- ✅ Set up development environment
- ✅ Verified KVM access
- ✅ Installed Rust toolchain
- ✅ Built NanoLambda project

**You're ready to start coding!** 🎉

Proceed to:
- **Week 1, Day 3-4:** KVM Integration tasks
- See `docs/04-roadmap.md` for detailed tasks

---

**Questions?** Open an issue on GitHub or check the FAQ.

**Document Version:** 1.0  
**Last Updated:** October 6, 2025
