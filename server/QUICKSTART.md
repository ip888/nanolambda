# Getting Started with NanoLambda Development

**Welcome!** This guide will get you from zero to running code in 30 minutes.

---

## 📋 What You Just Got

I've created a complete project structure with:

✅ **Documentation** (8 comprehensive docs)
- Executive summary with business strategy
- Market analysis with customer personas  
- Technical architecture diagrams
- 4-month development roadmap
- Setup guide for cloud development
- And more...

✅ **Rust Project Structure**
- Workspace with 5 crates (vmm, api, runtime, scheduler, storage)
- Server binary (main application)
- CLI binary (command-line tool)
- Tests framework
- Proper error handling

✅ **Development Tools**
- Cargo configuration
- Git ignore rules
- Contributing guidelines
- Changelog template

---

## 🚨 IMPORTANT: Next Steps (READ THIS!)

### Step 1: Understand the M1 Mac Limitation

**Your MacBook Air M1 CANNOT run KVM natively.**

You have 3 options:

**A) GitHub Codespaces** (Recommended - Start Today!)
- Cost: ~$30/month
- Setup time: 5 minutes
- Works from your M1 Mac

**B) AWS EC2 Instance**
- Cost: ~$35/month
- Setup time: 20 minutes
- More control

**C) Buy/Rent Intel Machine**
- Upfront cost: $200-400
- Or Hetzner server: $40/month

👉 **I recommend Option A (Codespaces) to start immediately.**

---

## 🎯 Your First Day Tasks

### 1. Set Up Cloud Development Environment (30 min)

Follow the detailed guide:
```bash
cd /Users/igor/c
cat docs/setup-guide.md
```

**Quick Start with GitHub Codespaces:**

1. **Push this code to GitHub:**
   ```bash
   cd /Users/igor/c
   git init
   git add .
   git commit -m "Initial NanoLambda project structure"
   
   # Create repo on GitHub, then:
   git remote add origin https://github.com/yourusername/nanolambda.git
   git branch -M main
   git push -u origin main
   ```

2. **Create Codespace:**
   - Go to your GitHub repo
   - Click "Code" → "Codespaces" → "New codespace"
   - Select: 4-core, 8GB RAM
   - Wait 2-3 minutes

3. **Verify KVM in Codespace:**
   ```bash
   sudo apt update
   sudo apt install -y qemu-kvm libvirt-daemon-system cpu-checker
   sudo kvm-ok  # Should say: KVM acceleration can be used
   ```

4. **Install Rust:**
   ```bash
   curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
   source $HOME/.cargo/env
   rustc --version  # Verify
   ```

5. **Build the Project:**
   ```bash
   cd /workspaces/nanolambda
   cargo build
   # First build takes 5-10 minutes (downloads dependencies)
   ```

---

### 2. Read the Documentation (1-2 hours)

**Priority order:**

1. **`docs/00-executive-summary.md`** (15 min)
   - Understand the vision
   - See revenue projections
   - Review competitive analysis

2. **`docs/04-roadmap.md`** (30 min)
   - See the 4-month plan
   - Understand weekly milestones
   - Note: Week 1 tasks start on Day 3 (you're on Day 2 now!)

3. **`docs/02-technical-architecture.md`** (45 min)
   - Understand system design
   - Review code examples
   - See performance targets

4. **`docs/01-market-analysis.md`** (30 min, optional now)
   - Customer personas
   - Market sizing
   - Go-to-market strategy

---

### 3. Understand the Codebase (30 min)

**Project Structure:**
```
/Users/igor/c/
├── docs/                  # All documentation (READ THESE!)
├── src/
│   ├── lib.rs            # Main library entry
│   └── bin/
│       ├── server.rs     # API server binary
│       └── cli.rs        # CLI tool binary
├── crates/               # Workspace crates
│   ├── vmm/              # ⭐ Core: MicroVM manager (YOU'LL SPEND MOST TIME HERE)
│   ├── api-server/       # REST API (Month 2)
│   ├── runtime/          # Python/Node/Java runtimes (Month 1-2)
│   ├── scheduler/        # Orchestration (Month 2)
│   └── storage/          # Function registry (Month 2)
└── Cargo.toml            # Workspace config
```

**Current State:**
- ✅ Project structure created
- ✅ Dependencies configured
- ✅ Skeleton code with TODOs
- ❌ No functionality yet (that's your job!)

---

### 4. Run the Placeholder Server (5 min)

```bash
# In your cloud environment:
cd /workspaces/nanolambda  # or ~/nanolambda

# Run the server
cargo run --bin nanolambda-server

# Should see:
# Starting NanoLambda Server v0.1.0
# ========================================
# Initializing VMM...
# ...
# API endpoint: http://localhost:8080
```

**Try the CLI:**
```bash
cargo run --bin nanolambda-cli -- list
# Should see: "Listing functions:" (no functions yet)
```

---

## 📅 Your Week 1 Roadmap

### Day 1-2: Setup & Documentation ✅ DONE!
- [x] Project structure created
- [x] Documentation written
- [ ] Cloud environment ready (YOUR NEXT TASK!)

### Day 3-4: KVM Integration (Starting Tomorrow!)
**Goal:** Create a VM and get KVM file descriptor

**Tasks:**
1. Read KVM documentation:
   - https://www.linux-kvm.org/page/Documents
   - https://docs.rs/kvm-ioctls/

2. Study Firecracker's VMM:
   - https://github.com/firecracker-microvm/firecracker/tree/main/src/vmm

3. Implement in `crates/vmm/src/vm.rs`:
   ```rust
   // Add real KVM initialization
   // Create vCPUs
   // Allocate guest memory
   ```

4. Write test:
   ```bash
   cargo test --package nanolambda-vmm
   ```

**Success Criteria:**
- ✅ `/dev/kvm` opens successfully
- ✅ VM created without errors
- ✅ vCPU initialized
- ✅ 128MB guest memory allocated

---

## 🎓 Learning Resources

### Must-Read Before Coding

**KVM Basics:**
- [KVM API Documentation](https://www.kernel.org/doc/Documentation/virtual/kvm/api.txt)
- [kvm-ioctls Rust Crate](https://docs.rs/kvm-ioctls/)

**Firecracker (Your Inspiration):**
- [Firecracker Design Doc](https://github.com/firecracker-microvm/firecracker/blob/main/docs/design.md)
- [Firecracker VMM Code](https://github.com/firecracker-microvm/firecracker/tree/main/src/vmm)

**Rust Async (for Month 2):**
- [Tokio Tutorial](https://tokio.rs/tokio/tutorial)
- [Async Book](https://rust-lang.github.io/async-book/)

---

## 🐛 Common Issues & Solutions

### Issue: "kvm-ok says KVM not available"
**Solution:** You're on your M1 Mac. You MUST use cloud environment.

### Issue: "Cargo build fails with linking errors"
**Solution:**
```bash
sudo apt install -y build-essential pkg-config libssl-dev
cargo clean && cargo build
```

### Issue: "Permission denied on /dev/kvm"
**Solution:**
```bash
sudo usermod -aG kvm $USER
# Log out and back in
```

---

## 💡 Pro Tips

### 1. Use VS Code Remote
Connect VS Code to your Codespace:
- Install "Remote - SSH" extension
- Or use "GitHub Codespaces" extension
- Full IDE experience, remote execution

### 2. Keep Notes
Document your learnings:
```bash
# Create a dev journal
touch DEVLOG.md
# Track daily progress, blockers, learnings
```

### 3. Commit Often
```bash
git add .
git commit -m "Day 3: Implemented KVM initialization"
git push
```

### 4. Test Frequently
```bash
# Run tests after every change
cargo test

# Run specific test
cargo test --package nanolambda-vmm test_vm_creation
```

---

## 📊 Success Milestones

### Week 1 (Oct 6-12)
- [ ] Cloud environment working
- [ ] KVM opens successfully
- [ ] VM created
- [ ] Guest memory allocated
- [ ] Kernel loaded (stretch goal)

### Week 2 (Oct 13-19)
- [ ] Minimal kernel boots
- [ ] Serial console working
- [ ] Execute shell command in VM

### Week 3 (Oct 20-26)
- [ ] Python runtime functional
- [ ] Execute Python function
- [ ] Cold start <100ms

### Week 4 (Oct 27 - Nov 2)
- [ ] Snapshot/restore working
- [ ] Cold start <10ms
- [ ] Month 1 complete! 🎉

---

## 🤔 Questions? Stuck?

### Resources
1. **Documentation:** Check `docs/` folder first
2. **Code Comments:** Read TODOs in source files
3. **Firecracker:** Study their implementation
4. **Rust Docs:** https://doc.rust-lang.org/

### Debugging Strategy
1. Read error messages carefully
2. Check you're in correct environment (cloud, not M1)
3. Verify KVM access (`sudo kvm-ok`)
4. Add `println!` debug statements
5. Use `RUST_LOG=debug cargo run`

---

## 🎯 Your Immediate Next Steps (Right Now!)

1. **Set up GitHub repository** (10 min)
   ```bash
   cd /Users/igor/c
   # Follow git init instructions above
   ```

2. **Create GitHub Codespace** (5 min)
   - Push code to GitHub
   - Create Codespace
   - Install tools

3. **Build the project** (10 min)
   ```bash
   cargo build
   cargo test
   ```

4. **Read Week 1 tasks in roadmap** (30 min)
   ```bash
   cat docs/04-roadmap.md
   # Focus on Day 3-4 tasks
   ```

5. **Start coding!** (Rest of the week)
   - Open `crates/vmm/src/vm.rs`
   - Read the TODOs
   - Implement KVM initialization
   - Test frequently

---

## 🎉 You're Ready!

You now have:
- ✅ Complete project structure
- ✅ 4-month roadmap
- ✅ Technical architecture
- ✅ Market strategy
- ✅ Clear next steps

**The hard part (planning) is done. Now it's time to build!**

---

## 📞 Final Checklist

Before you start coding, ensure:

- [ ] I've read the executive summary
- [ ] I understand the 4-month roadmap
- [ ] I've set up cloud development environment
- [ ] KVM is working (`sudo kvm-ok`)
- [ ] Rust is installed (`rustc --version`)
- [ ] Project builds (`cargo build`)
- [ ] I know what to build next (Week 1, Day 3-4 tasks)

**All checked?** You're ready to build the future of serverless! 🚀

---

**Good luck! You're building something amazing.** 💪

Remember: Focus on Python-only runtime first. Java can wait. Ship fast, iterate based on feedback.

**Questions or stuck?** Re-read the docs or check Firecracker's code for inspiration.
