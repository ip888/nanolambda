# Contributing to NanoLambda

## Development Setup

```bash
# Clone
git clone https://github.com/ip888/nanolambda.git
cd nanolambda/server

# Build and test
cargo build
cargo test --workspace

# Run locally
cargo run --bin nanolambda-server
```

**Requirements**: Rust 1.93+, Python 3.11+, Linux (for full sandbox isolation).

## Code Quality

```bash
cargo fmt --all -- --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test --workspace
```

Or use the pre-push check: `../scripts/pre-push-check.sh`

## Pull Request Process

1. Fork the repository
2. Create a feature branch (`git checkout -b feature/amazing-feature`)
3. Make changes, add tests
4. Run `../scripts/pre-push-check.sh`
5. Commit and push
6. Open a Pull Request

## Standards

- **Rust Edition 2024**, `rust-version = "1.93"`
- **Clippy pedantic** with `-D warnings`
- Tests for new functionality
- No secrets or `.env` files in commits
