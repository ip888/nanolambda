# NanoLambda Platform

> **High-performance serverless computing: Self-hosted infrastructure and Edge deployment**

[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)
[![Rust](https://img.shields.io/badge/rust-1.93+-orange.svg)](https://www.rust-lang.org/)
[![Edition](https://img.shields.io/badge/edition-2024-blue.svg)](https://doc.rust-lang.org/edition-guide/rust-2024/)

---

## 📦 Project Structure

This monorepo contains two independent projects designed for parallel development:

```
nanolambda/
├── server/     # Traditional self-hosted serverless platform
├── edge/       # Edge computing platform (Cloudflare Workers)
├── LICENSE
├── CHANGELOG.md
├── CONTRIBUTING.md
└── README.md   # This file
```

---

## 🖥️ Server Platform (`/server`)

**Self-hosted AWS Lambda-compatible serverless platform with microVM isolation**

### Features

- ✅ **~0ms warm starts** - 10-50x faster than AWS Lambda
- ✅ **~32ms cold starts** - 3-10x faster than AWS Lambda
- ✅ **Process pooling** - Instant execution after first invocation
- ✅ **Function versioning** - AWS Lambda-compatible versioning system
- ✅ **API key authentication** - Secure access with Bearer tokens
- ✅ **AWS Lambda API compatibility** - Drop-in replacement
- ✅ **MicroVM isolation** - Hardware-backed security (coming soon)
- ✅ **70% cost reduction** - Run on your own hardware
- ✅ **Multi-language support** - Python (Node.js, Java coming soon)

### Quick Start

```bash
cd server
cargo build --release
cargo test
cargo run
```

### Documentation

See [server/docs/](server/docs/) for detailed documentation.

---

## 🌐 Edge Platform (`/edge`)

**AI-powered Edge computing platform deployed on Cloudflare Workers**

### Features

- ✅ **Semantic Caching** - AI-powered response caching with similarity matching
- ✅ **Hybrid Search** - Combined vector + BM25 full-text search
- ✅ **Reranking** - ML-based result relevance optimization
- ✅ **Global Edge Deployment** - 300+ Cloudflare locations worldwide
- ✅ **Sub-10ms latency** - Cold starts under 10ms
- ✅ **Zero infrastructure** - No servers to manage

### Quick Start

```bash
cd edge
npm install
npx wrangler dev       # Local development
npx wrangler deploy    # Production deployment
```

### Documentation

See [edge/README.md](edge/README.md) for detailed documentation.

---

## 🛠️ Development

### Prerequisites

| Platform   | Requirements                                               |
| ---------- | ---------------------------------------------------------- |
| **Server** | Linux x86_64 with KVM, Rust 1.93+, 4GB+ RAM                |
| **Edge**   | Node.js 18+, Rust 1.93+ with wasm32-unknown-unknown target |

### Code Quality Standards

Both projects enforce strict Rust quality standards:

- **Rust Edition 2024** with `rust-version = "1.93"`
- **Clippy pedantic** lints enabled
- **Panic safety** - All fallible operations documented
- **Comprehensive documentation** - Business and technical decisions explained

### Building Both Projects

```bash
# Server platform
cd server && cargo build --release && cargo test

# Edge platform
cd edge && cargo build --target wasm32-unknown-unknown --release
```

---

## 📄 License

MIT License - see [LICENSE](LICENSE) for details.

---

## 🤝 Contributing

See [CONTRIBUTING.md](CONTRIBUTING.md) for contribution guidelines.
