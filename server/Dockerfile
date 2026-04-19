# =============================================================================
# NanoLambda Server - Production Multi-Stage Dockerfile
# =============================================================================
#
# Build: docker build -t nanolambda-server .
# Run:   docker run -p 8080:8080 -v /data:/data nanolambda-server
#
# Environment Variables:
#   DATABASE_PATH  SQLite database location (default: /data/nanolambda.db)
#   RUST_LOG       Logging level (default: info,nanolambda=debug)
#   PORT           HTTP server port (default: 8080)
# =============================================================================

# -----------------------------------------------------------------------------
# Stage 1: Build Dependencies (cached layer)
# -----------------------------------------------------------------------------
FROM rust:1.93-slim AS deps

WORKDIR /app

RUN apt-get update && apt-get install -y --no-install-recommends \
    pkg-config \
    libssl-dev \
    && rm -rf /var/lib/apt/lists/*

RUN cargo new --bin nanolambda-server
WORKDIR /app/nanolambda-server

# Copy dependency manifests
COPY Cargo.toml Cargo.lock ./
COPY crates/api-server/Cargo.toml crates/api-server/Cargo.toml
COPY crates/mcp/Cargo.toml crates/mcp/Cargo.toml
COPY crates/runtime/Cargo.toml crates/runtime/Cargo.toml
COPY crates/storage/Cargo.toml crates/storage/Cargo.toml
COPY benchmarks/Cargo.toml benchmarks/Cargo.toml

# Create dummy lib files for workspace members
RUN mkdir -p crates/api-server/src && echo "pub fn dummy() {}" > crates/api-server/src/lib.rs
RUN mkdir -p crates/mcp/src && echo "fn main() {}" > crates/mcp/src/main.rs
RUN mkdir -p crates/runtime/src && echo "pub fn dummy() {}" > crates/runtime/src/lib.rs
RUN mkdir -p crates/storage/src && echo "pub fn dummy() {}" > crates/storage/src/lib.rs
RUN mkdir -p benchmarks/src && echo "fn main() {}" > benchmarks/src/main.rs

# Build dependencies only (this layer is cached)
RUN cargo build --release 2>/dev/null || true
RUN rm -rf src crates benchmarks

# -----------------------------------------------------------------------------
# Stage 2: Build Application
# -----------------------------------------------------------------------------
FROM deps AS builder

WORKDIR /app/nanolambda-server

COPY . .

# Invalidate cached dummy crate artifacts so the real source is compiled
RUN find crates src benchmarks -name '*.rs' -exec touch {} +

RUN cargo build --release --bin nanolambda-server && \
    cargo build --release -p nanolambda-mcp --bin nanolambda-mcp

RUN ls -la target/release/nanolambda-server target/release/nanolambda-mcp

# -----------------------------------------------------------------------------
# Stage 3: Runtime Image
# -----------------------------------------------------------------------------
FROM debian:bookworm-slim AS runtime

LABEL org.opencontainers.image.title="NanoLambda Server"
LABEL org.opencontainers.image.description="AI-agent code-execution sandbox — fast, cheap, self-hostable"
LABEL org.opencontainers.image.version="0.1.0"
LABEL org.opencontainers.image.vendor="NanoLambda"
LABEL org.opencontainers.image.source="https://github.com/ip888/nanolambda"

# Python 3.12 — pinned for production stability
RUN apt-get update && apt-get install -y --no-install-recommends \
    ca-certificates \
    python3 \
    python3-pip \
    python3-venv \
    curl \
    tini \
    && rm -rf /var/lib/apt/lists/* \
    && ln -sf /usr/bin/python3 /usr/bin/python \
    && python3 --version | grep -q "3.12" || echo "WARNING: expected Python 3.12"

# Non-root user
RUN groupadd --gid 1000 nanolambda \
    && useradd --uid 1000 --gid nanolambda --shell /bin/bash --create-home nanolambda

WORKDIR /app

# Copy binaries
COPY --from=builder /app/nanolambda-server/target/release/nanolambda-server /usr/local/bin/
COPY --from=builder /app/nanolambda-server/target/release/nanolambda-mcp /usr/local/bin/

# Data directories
RUN mkdir -p /data /tmp/nanolambda/sandbox \
    && chown -R nanolambda:nanolambda /data /app /tmp/nanolambda

ENV NANOLAMBDA_DB_PATH=/data/nanolambda.db
ENV RUST_LOG=info,nanolambda=debug
ENV PORT=8080
ENV HOST=0.0.0.0

EXPOSE 8080

HEALTHCHECK --interval=30s --timeout=5s --start-period=10s --retries=3 \
    CMD curl -f http://localhost:${PORT}/health || exit 1

USER nanolambda

ENTRYPOINT ["/usr/bin/tini", "--"]
CMD ["nanolambda-server"]
