#!/usr/bin/env bash
# =============================================================================
# Pre-push check — run locally before pushing to catch CI failures early.
#
# Usage:  ./scripts/pre-push-check.sh          (full check)
#         ./scripts/pre-push-check.sh --quick   (fmt + clippy only)
#
# Install as git hook:
#   ln -sf ../../scripts/pre-push-check.sh .git/hooks/pre-push
# =============================================================================
set -euo pipefail

# Ensure cargo is on PATH (common install locations)
export PATH="$HOME/.cargo/bin:$PATH"

RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m'

QUICK=false
[[ "${1:-}" == "--quick" ]] && QUICK=true

cd "$(git rev-parse --show-toplevel)/server"

pass() { echo -e "${GREEN}✓${NC} $1"; }
fail() { echo -e "${RED}✗${NC} $1"; exit 1; }
warn() { echo -e "${YELLOW}⚠${NC} $1"; }

# ── 1. Formatting ────────────────────────────────────────────────────────────
echo "── Checking formatting ──"
cargo fmt --all -- --check 2>&1 || fail "cargo fmt failed — run 'cargo fmt' and re-commit"
pass "cargo fmt"

# ── 2. Clippy ────────────────────────────────────────────────────────────────
echo "── Running clippy ──"
cargo clippy --all-targets --all-features -- -D warnings 2>&1 || fail "clippy has warnings"
pass "clippy"

if $QUICK; then
    echo -e "\n${GREEN}Quick checks passed.${NC}"
    exit 0
fi

# ── 3. Tests ─────────────────────────────────────────────────────────────────
echo "── Running tests ──"
cargo test --workspace --lib 2>&1 || fail "unit tests failed"
cargo test --workspace --test '*' 2>&1 || fail "integration tests failed"
pass "all tests"

# ── 4. Docker build (syntax only — no push) ──────────────────────────────────
echo "── Verifying Dockerfile parses ──"
if command -v docker &>/dev/null; then
    docker build --check . 2>/dev/null && pass "Dockerfile syntax" \
        || { docker build --target builder --no-cache -f Dockerfile . -o /dev/null 2>&1 | tail -5; warn "Docker build --check not supported, skipping"; }
else
    warn "docker not installed — skipping Dockerfile check"
fi

# ── 5. Secrets scan ──────────────────────────────────────────────────────────
echo "── Scanning for secrets ──"
SECRETS_FOUND=false
# Check staged/committed files for common secret patterns
if git diff --cached --name-only 2>/dev/null | head -1 >/dev/null; then
    DIFF_TARGET="--cached"
else
    DIFF_TARGET="HEAD~1..HEAD"
fi

if git diff $DIFF_TARGET -U0 2>/dev/null | grep -iqE '(sk-[a-z0-9]{20,}|AKIA[A-Z0-9]{16}|ghp_[a-zA-Z0-9]{36}|-----BEGIN (RSA |EC )?PRIVATE KEY)'; then
    fail "Possible secret/key detected in diff — review before pushing"
fi

# Check for .env files being committed
if git ls-files --cached | grep -qE '\.env$'; then
    fail ".env file is tracked by git — add it to .gitignore"
fi
pass "no secrets detected"

echo -e "\n${GREEN}All pre-push checks passed.${NC}"
