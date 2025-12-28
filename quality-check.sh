#!/bin/bash
# Quality check script for nanolambda
# Runs all code quality verification steps

set -e

RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

echo "🚀 Running Nanolambda Quality Checks"
echo "===================================="
echo ""

# Format check
echo "🔍 Checking code formatting..."
if cargo fmt -- --check; then
    echo -e "${GREEN}✅ Format check passed${NC}"
else
    echo -e "${RED}❌ Format check failed - run 'cargo fmt' to fix${NC}"
    exit 1
fi
echo ""

# Clippy standard strict
echo "📋 Running clippy (standard strict)..."
if cargo clippy --all-targets --all-features -- -D warnings; then
    echo -e "${GREEN}✅ Clippy passed (standard strict)${NC}"
else
    echo -e "${RED}❌ Clippy failed${NC}"
    exit 1
fi
echo ""

# Tests
echo "🧪 Running tests..."
if cargo test --all-features --workspace --lib; then
    echo -e "${GREEN}✅ Tests passed${NC}"
else
    echo -e "${RED}❌ Tests failed${NC}"
    exit 1
fi
echo ""

# Build release
echo "📦 Building release..."
if cargo build --release --all-features; then
    echo -e "${GREEN}✅ Release build successful${NC}"
else
    echo -e "${RED}❌ Release build failed${NC}"
    exit 1
fi
echo ""

echo "===================================="
echo -e "${GREEN}✅ All quality checks passed!${NC}"
echo ""
echo "Optional stricter checks:"
echo "  - Ultra-strict clippy: cargo clippy -- -W clippy::pedantic -W clippy::nursery"
echo "  - Security audit: cargo audit (requires: cargo install cargo-audit)"
echo "  - Coverage: cargo tarpaulin (requires: cargo install cargo-tarpaulin)"
echo ""
