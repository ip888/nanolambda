# Rust Production Build Best Practices

This document summarizes the build optimizations applied to NanoLambda for production readiness.

## Build Time Optimizations

### 1. Mold Linker (10-20x faster linking)
```toml
# .cargo/config.toml
[target.x86_64-unknown-linux-gnu]
linker = "clang"
rustflags = ["-C", "link-arg=-fuse-ld=mold"]
```

Install: `sudo apt install mold clang`

### 2. Incremental Compilation
```toml
# Cargo.toml
[profile.dev]
incremental = true  # Enabled by default
```

### 3. Optimized Dependencies in Dev Mode
```toml
[profile.dev.package."*"]
opt-level = 2  # Optimize deps for faster runtime (one-time cost)
```

### 4. Reduced Debug Info
```toml
[profile.dev]
debug = 1  # Line tables only (faster than debug = 2)
```

## Build Profiles

| Profile | Use Case | Build Time | Binary Size | Performance |
|---------|----------|------------|-------------|-------------|
| `dev` | Local development | Fastest | Largest | Slowest |
| `test` | Running tests | Fast | Large | Good |
| `release-fast` | CI builds | Moderate | Small | Fast |
| `release` | Production | Slow | Smallest | Fastest |
| `release-small` | Constrained envs | Slow | Smallest | Fast |

### Usage
```bash
cargo build                      # dev profile
cargo build --release            # full optimization
cargo build --profile release-fast  # faster CI builds
cargo build --profile release-small # minimum size
```

## Binary Size Optimizations

### Profile Settings
```toml
[profile.release]
opt-level = 3        # Maximum speed optimization
lto = true           # Link-Time Optimization
codegen-units = 1    # Best optimization (slower build)
strip = true         # Remove debug symbols
panic = "abort"      # No unwinding (smaller binary)

[profile.release-small]
opt-level = "z"      # Optimize for size
lto = true
codegen-units = 1
strip = true
panic = "abort"
```

### Results

| Binary | Standard Release | Size-Optimized |
|--------|-----------------|----------------|
| nanolambda-cli | 2.3 MB | 1.8 MB (-22%) |
| nanolambda-server | 8.8 MB | 6.1 MB (-31%) |

## Dependency Optimization

### Tokio Feature Trimming
```toml
# Only include needed features instead of "full"
tokio = { version = "1.45", features = [
    "rt-multi-thread",  # Multi-threaded runtime
    "net",              # TCP/UDP networking
    "io-util",          # I/O utilities
    "time",             # Timers
    "sync",             # Synchronization primitives
    "macros",           # #[tokio::main], etc.
    "signal",           # Signal handling
    "fs"                # File system operations
] }
```

## Pre-Push Checklist

Before pushing code, run:
```bash
# Format check
cargo fmt --check

# Lint check
cargo clippy -- -D warnings

# Run all tests
cargo test --workspace

# Build release
cargo build --release

# Generate documentation
cargo doc --no-deps
```

## Continuous Integration

### Recommended CI Profile
Use `release-fast` for CI builds to balance speed and quality:
```bash
cargo build --profile release-fast
cargo test --profile release-fast
```

### Caching
Cache these directories:
- `~/.cargo/registry/`
- `~/.cargo/git/`
- `target/`

## Monitoring Build Times

```bash
# Time a clean build
cargo clean && time cargo build --release

# Analyze build times
cargo build --timings
# Opens target/cargo-timings/cargo-timing.html
```

## Additional Resources

- [The Cargo Book - Profiles](https://doc.rust-lang.org/cargo/reference/profiles.html)
- [Min-Sized Rust](https://github.com/johnthagen/min-sized-rust)
- [Rust Performance Book](https://nnethercote.github.io/perf-book/)
