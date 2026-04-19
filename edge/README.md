# NanoLambda Edge

A high-performance, Rust-based edge serverless platform for Cloudflare Workers.

## Overview

NanoLambda Edge is a complete serverless platform implementation in Rust, compiled to WebAssembly for deployment on Cloudflare Workers. It provides:

- **Function Management**: CRUD operations for serverless functions
- **Function Invocation**: Execute JavaScript functions at the edge (Python support planned)
- **Vector Database (QuartzDB)**: Built-in HNSW vector index for AI/ML workloads
- **Workers AI Integration**: Embeddings, completions, and chat via Cloudflare AI
- **Authentication**: API Key and JWT-based authentication
- **Rate Limiting**: Built-in rate limiting via Durable Objects
- **Session Management**: Persistent sessions via Durable Objects

## Architecture

```
┌─────────────────────────────────────────────────────────────────────┐
│                     Cloudflare Workers Edge                          │
├─────────────────────────────────────────────────────────────────────┤
│  ┌───────────────────────────────────────────────────────────────┐  │
│  │                   NanoLambda Edge (Rust/WASM)                 │  │
│  │  ┌─────────────┐  ┌─────────────┐  ┌─────────────┐           │  │
│  │  │   Router    │  │    Auth     │  │   Handlers  │           │  │
│  │  └─────────────┘  └─────────────┘  └─────────────┘           │  │
│  │  ┌─────────────┐  ┌─────────────┐  ┌─────────────┐           │  │
│  │  │  Functions  │  │   Vectors   │  │     AI      │           │  │
│  │  └─────────────┘  └─────────────┘  └─────────────┘           │  │
│  └───────────────────────────────────────────────────────────────┘  │
├─────────────────────────────────────────────────────────────────────┤
│                      Cloudflare Bindings                             │
│  ┌───────────┐  ┌───────────┐  ┌───────────┐  ┌───────────┐        │
│  │ KV Store  │  │  Durable  │  │ R2 Bucket │  │ Workers   │        │
│  │           │  │  Objects  │  │           │  │    AI     │        │
│  └───────────┘  └───────────┘  └───────────┘  └───────────┘        │
└─────────────────────────────────────────────────────────────────────┘
```

## Quick Start

### Prerequisites

- Rust 1.93+ with `wasm32-unknown-unknown` target
- Node.js 18+
- Cloudflare account (for deployment)

### Local Development

```bash
# Install dependencies
npm install

# Start local development server
npm run dev

# Or with explicit local mode
npx wrangler dev --local

# Run tests
./test-edge.sh
```

### Build

```bash
# Build WASM binary
npm run build

# Output in build/ directory:
# - index.js (~30KB)
# - index_bg.wasm (~718KB)
```

### Deploy

```bash
# Deploy to Cloudflare Workers
npm run deploy
```

## API Reference

### Health Check

```bash
curl http://localhost:8787/health
# {"status":"healthy","version":"0.1.0","region":"unknown","timestamp":"..."}
```

### Functions

```bash
# Create function
curl -X POST http://localhost:8787/v1/functions \
  -H "Content-Type: application/json" \
  -H "X-Api-Key: dev-test-key" \
  -d '{
    "name": "hello-world",
    "runtime": "javascript",
    "code": "export default { fetch() { return new Response(\"Hello!\"); } }"
  }'

# List functions
curl http://localhost:8787/v1/functions \
  -H "X-Api-Key: dev-test-key"

# Get function
curl http://localhost:8787/v1/functions/{id} \
  -H "X-Api-Key: dev-test-key"

# Update function
curl -X PUT http://localhost:8787/v1/functions/{id} \
  -H "Content-Type: application/json" \
  -H "X-Api-Key: dev-test-key" \
  -d '{"code": "..."}'

# Delete function
curl -X DELETE http://localhost:8787/v1/functions/{id} \
  -H "X-Api-Key: dev-test-key"

# Invoke function
curl -X POST http://localhost:8787/v1/functions/{id}/invoke \
  -H "Content-Type: application/json" \
  -H "X-Api-Key: dev-test-key" \
  -d '{"payload": {"name": "World"}}'
```

### Vectors (QuartzDB)

```bash
# Upsert vectors
curl -X POST http://localhost:8787/v1/vectors/upsert \
  -H "Content-Type: application/json" \
  -H "X-Api-Key: dev-test-key" \
  -d '{
    "namespace": "my-vectors",
    "vectors": [
      {"id": "vec1", "values": [0.1, 0.2, 0.3], "metadata": {"text": "hello"}}
    ]
  }'

# Query vectors
curl -X POST http://localhost:8787/v1/vectors/query \
  -H "Content-Type: application/json" \
  -H "X-Api-Key: dev-test-key" \
  -d '{
    "namespace": "my-vectors",
    "vector": [0.1, 0.2, 0.3],
    "top_k": 5,
    "include_metadata": true
  }'

# Delete vectors
curl -X POST http://localhost:8787/v1/vectors/delete \
  -H "Content-Type: application/json" \
  -H "X-Api-Key: dev-test-key" \
  -d '{"namespace": "my-vectors", "ids": ["vec1"]}'

# Get stats
curl http://localhost:8787/v1/vectors/stats \
  -H "X-Api-Key: dev-test-key"
```

### AI (requires Cloudflare auth)

```bash
# Generate embeddings
curl -X POST http://localhost:8787/v1/ai/embeddings \
  -H "Content-Type: application/json" \
  -H "X-Api-Key: dev-test-key" \
  -d '{
    "input": "Hello, world!",
    "model": "@cf/baai/bge-small-en-v1.5"
  }'

# Text completion
curl -X POST http://localhost:8787/v1/ai/completions \
  -H "Content-Type: application/json" \
  -H "X-Api-Key: dev-test-key" \
  -d '{
    "prompt": "The meaning of life is",
    "model": "@cf/meta/llama-2-7b-chat-int8"
  }'

# Chat
curl -X POST http://localhost:8787/v1/ai/chat \
  -H "Content-Type: application/json" \
  -H "X-Api-Key: dev-test-key" \
  -d '{
    "messages": [{"role": "user", "content": "Hello!"}],
    "model": "@cf/meta/llama-2-7b-chat-int8"
  }'
```

## Authentication

### Development Mode

In development mode (`ENVIRONMENT=development`), use API keys starting with `dev-`:

```bash
curl -H "X-Api-Key: dev-test-key" ...
```

### Production Mode

In production, API keys must be registered in the `API_KEYS` KV namespace:

```bash
# Store API key (hashed with SHA256)
wrangler kv:put --binding API_KEYS "<key_hash>" '{"owner_id":"user1","permissions":["functions_read","functions_write"],"rate_limit":{"requests_per_minute":60}}'
```

### JWT Authentication

Bearer tokens are also supported:

```bash
curl -H "Authorization: Bearer <jwt_token>" ...
```

## Configuration

### wrangler.toml

```toml
name = "nanolambda-edge"
main = "build/worker/shim.mjs"
compatibility_date = "2024-01-01"

# KV Namespaces
[[kv_namespaces]]
binding = "FUNCTIONS"
id = "<your-kv-id>"

[[kv_namespaces]]
binding = "API_KEYS"
id = "<your-kv-id>"

# Durable Objects
[durable_objects]
bindings = [
    { name = "VECTOR_INDEX", class_name = "VectorIndex" },
    { name = "USER_SESSION", class_name = "UserSession" },
    { name = "RATE_LIMITER", class_name = "RateLimiter" }
]

# R2 Bucket
[[r2_buckets]]
binding = "PACKAGES"
bucket_name = "nanolambda-packages"

# Workers AI
[ai]
binding = "AI"
```

## Project Structure

```
crates/edge/
├── Cargo.toml           # Rust dependencies
├── wrangler.toml        # Cloudflare Workers config
├── package.json         # npm scripts
├── test-edge.sh         # Test script
└── src/
    ├── lib.rs           # Entry point, Durable Objects
    ├── router.rs        # HTTP request routing
    ├── auth.rs          # Authentication middleware
    ├── error.rs         # Error types
    ├── types.rs         # Type definitions
    ├── handlers/
    │   ├── mod.rs
    │   ├── functions.rs # Function CRUD
    │   ├── invoke.rs    # Function invocation
    │   ├── vectors.rs   # Vector operations
    │   ├── ai.rs        # AI endpoints
    │   ├── session.rs   # Session management
    │   ├── rate_limit.rs# Rate limiting
    │   └── usage.rs     # Usage tracking
    └── vector/
        ├── mod.rs
        ├── hnsw.rs      # HNSW vector index
        └── durable_object.rs  # DO storage
```

## Tech Stack

- **Rust**: Core implementation
- **workers-rs 0.7**: Cloudflare Workers SDK
- **WebAssembly**: Compilation target
- **wrangler**: Build and deployment tool

### Key Dependencies

| Crate | Version | Purpose |
|-------|---------|---------|
| worker | 0.7 | Cloudflare Workers SDK |
| serde | 1.0 | Serialization |
| chrono | 0.4 | Time handling (wasmbind) |
| sha2 | 0.10 | API key hashing |
| hmac | 0.12 | JWT verification |
| uuid | 1.19 | ID generation (js feature) |
| rand | 0.9 | Vector operations |

## Performance

- **Binary Size**: ~718KB optimized WASM
- **Cold Start**: <5ms (WASM initialization)
- **Request Latency**: <20ms typical
- **Vector Operations**: Sub-millisecond for small indexes

## Roadmap

- [ ] Full JavaScript runtime via QuickJS WASM
- [ ] Python runtime via Pyodide
- [ ] Streaming AI responses
- [ ] Analytics Engine integration
- [ ] Custom domains support
- [ ] Cron triggers

## License

MIT License - see [LICENSE](../../LICENSE)
