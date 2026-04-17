# =============================================================================
# NanoLambda Server - Production Multi-Stage Dockerfile
# =============================================================================
# 
# Build: docker build -t nanolambda-server .
# Run:   docker run -p 8080:8080 -v /data:/data nanolambda-server
#
# Requirements:
# - Linux host with KVM support for MicroVM features
# - At least 2GB RAM recommended
#
# Environment Variables:
# - DATABASE_PATH: SQLite database location (default: /data/nanolambda.db)
# - RUST_LOG: Logging level (default: info,nanolambda=debug)
# - PORT: HTTP server port (default: 8080)
# - METRICS_PORT: Prometheus metrics port (default: 9090)
# =============================================================================

# -----------------------------------------------------------------------------
# Stage 1: Build Dependencies (cached layer)
# -----------------------------------------------------------------------------
FROM rust:1.93-slim AS deps

WORKDIR /app

# Install build dependencies
RUN apt-get update && apt-get install -y --no-install-recommends \
    pkg-config \
    libssl-dev \
    && rm -rf /var/lib/apt/lists/*

# Create dummy project for dependency caching
RUN cargo new --bin nanolambda-server
WORKDIR /app/nanolambda-server

# Copy dependency manifests
COPY Cargo.toml Cargo.lock ./
COPY crates/api-server/Cargo.toml crates/api-server/Cargo.toml
COPY crates/runtime/Cargo.toml crates/runtime/Cargo.toml
COPY crates/storage/Cargo.toml crates/storage/Cargo.toml
COPY crates/vmm/Cargo.toml crates/vmm/Cargo.toml
COPY benchmarks/Cargo.toml benchmarks/Cargo.toml

# Create dummy lib files for workspace members
RUN mkdir -p crates/api-server/src && echo "pub fn dummy() {}" > crates/api-server/src/lib.rs
RUN mkdir -p crates/runtime/src && echo "pub fn dummy() {}" > crates/runtime/src/lib.rs
RUN mkdir -p crates/storage/src && echo "pub fn dummy() {}" > crates/storage/src/lib.rs
RUN mkdir -p crates/vmm/src && echo "pub fn dummy() {}" > crates/vmm/src/lib.rs
RUN mkdir -p benchmarks/src && echo "fn main() {}" > benchmarks/src/main.rs

# Build dependencies only (this layer is cached)
RUN cargo build --release 2>/dev/null || true
RUN rm -rf src crates benchmarks

# -----------------------------------------------------------------------------
# Stage 2: Build Application
# -----------------------------------------------------------------------------
FROM deps AS builder

WORKDIR /app/nanolambda-server

# Copy actual source code
COPY . .

# Build in release mode with optimizations
RUN cargo build --release --bin nanolambda-server

# Verify binary exists
RUN ls -la target/release/nanolambda-server

# -----------------------------------------------------------------------------
# Stage 3: Runtime Image
# -----------------------------------------------------------------------------
FROM debian:bookworm-slim AS runtime

# Labels for container metadata
LABEL org.opencontainers.image.title="NanoLambda Server"
LABEL org.opencontainers.image.description="Self-hosted serverless platform with hybrid VM/WASM isolation"
LABEL org.opencontainers.image.version="0.1.0"
LABEL org.opencontainers.image.vendor="NanoLambda"
LABEL org.opencontainers.image.source="https://github.com/ip888/nanolambda"

# Install runtime dependencies
RUN apt-get update && apt-get install -y --no-install-recommends \
    ca-certificates \
    python3 \
    python3-pip \
    nodejs \
    npm \
    curl \
    tini \
    && rm -rf /var/lib/apt/lists/* \
    && ln -s /usr/bin/python3 /usr/bin/python

# Create non-root user for security
RUN groupadd --gid 1000 nanolambda \
    && useradd --uid 1000 --gid nanolambda --shell /bin/bash --create-home nanolambda

WORKDIR /app

# Copy binary from builder
COPY --from=builder /app/nanolambda-server/target/release/nanolambda-server /usr/local/bin/nanolambda-server

# Create data directories
RUN mkdir -p /data /app/functions /app/logs \
    && chown -R nanolambda:nanolambda /data /app

# Set environment variables
ENV NANOLAMBDA_DB_PATH=/data/nanolambda.db
ENV RUST_LOG=info,nanolambda=debug
ENV PORT=8080
ENV HOST=0.0.0.0

# Expose ports
EXPOSE 8080 9090

# Health check with startup grace period
HEALTHCHECK --interval=30s --timeout=5s --start-period=10s --retries=3 \
    CMD curl -f http://localhost:${PORT}/health || exit 1

# Switch to non-root user
USER nanolambda

# Use tini for proper signal handling
ENTRYPOINT ["/usr/bin/tini", "--"]

# Run the server
CMD ["nanolambda-server"]
