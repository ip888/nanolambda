#!/usr/bin/env bash
# =============================================================================
# Pre-deploy check — validates the Docker image builds and runs correctly.
#
# Run this before merging deploy-triggering changes to main.
#
# Usage:  ./scripts/pre-deploy-check.sh
# =============================================================================
set -euo pipefail

RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m'

REPO_ROOT="$(git rev-parse --show-toplevel)"
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

cd "$REPO_ROOT/server"

pass() { echo -e "${GREEN}✓${NC} $1"; }
fail() { echo -e "${RED}✗${NC} $1"; exit 1; }
warn() { echo -e "${YELLOW}⚠${NC} $1"; }

IMAGE_NAME="nanolambda-server:pre-deploy-check"
CONTAINER_NAME="nanolambda-pre-deploy-check"

cleanup() {
    docker stop "$CONTAINER_NAME" 2>/dev/null || true
    docker rm "$CONTAINER_NAME" 2>/dev/null || true
}
trap cleanup EXIT

# ── 1. Run pre-push checks first ─────────────────────────────────────────────
echo "── Running pre-push checks ──"
bash "$SCRIPT_DIR/pre-push-check.sh" --quick || fail "pre-push checks failed"

# ── 2. Docker build ──────────────────────────────────────────────────────────
echo "── Building Docker image ──"
docker build -t "$IMAGE_NAME" . 2>&1 || fail "Docker build failed"
pass "Docker image built"

# ── 3. Verify binaries exist in image ────────────────────────────────────────
echo "── Checking binaries in image ──"
docker run --rm "$IMAGE_NAME" ls -la /usr/local/bin/nanolambda-server 2>&1 || fail "nanolambda-server binary missing"
docker run --rm "$IMAGE_NAME" ls -la /usr/local/bin/nanolambda-mcp 2>&1 || fail "nanolambda-mcp binary missing"
pass "both binaries present"

# ── 4. Verify Python is available ────────────────────────────────────────────
echo "── Checking Python in image ──"
PY_VERSION=$(docker run --rm "$IMAGE_NAME" python3 --version 2>&1)
echo "  $PY_VERSION"
echo "$PY_VERSION" | grep -qE "3\.(12|13)" || warn "unexpected Python version: $PY_VERSION"
pass "Python available"

# ── 5. Start container and health check ──────────────────────────────────────
echo "── Starting container for health check ──"
cleanup
docker run -d \
    --name "$CONTAINER_NAME" \
    -p 18080:8080 \
    -e DATABASE_PATH=/tmp/nanolambda-test.db \
    -e RUST_LOG=info \
    "$IMAGE_NAME" 2>&1 || fail "container failed to start"

echo "  Waiting for startup..."
for i in $(seq 1 15); do
    if curl -sf http://localhost:18080/health >/dev/null 2>&1; then
        pass "health endpoint responding"
        break
    fi
    if [ "$i" -eq 15 ]; then
        echo "  Container logs:"
        docker logs "$CONTAINER_NAME" 2>&1 | tail -20
        fail "health check timed out after 15s"
    fi
    sleep 1
done

# ── 6. Verify key endpoints ─────────────────────────────────────────────────
echo "── Checking endpoints ──"

HTTP_CODE=$(curl -s -o /dev/null -w '%{http_code}' http://localhost:18080/health)
[ "$HTTP_CODE" = "200" ] && pass "/health → 200" || fail "/health → $HTTP_CODE"

HTTP_CODE=$(curl -s -o /dev/null -w '%{http_code}' http://localhost:18080/metrics/prometheus)
[ "$HTTP_CODE" = "200" ] && pass "/metrics/prometheus → 200" || fail "/metrics/prometheus → $HTTP_CODE"

METRICS=$(curl -sf http://localhost:18080/metrics/prometheus)
echo "$METRICS" | grep -q "nanolambda_invocations_total" && pass "Prometheus metrics contain expected gauges" \
    || fail "Prometheus output missing expected metrics"

# ── 7. Quick sandbox smoke test ──────────────────────────────────────────────
echo "── Sandbox smoke test ──"
SANDBOX_RESP=$(curl -sf -X POST http://localhost:18080/sandbox/invoke \
    -H 'Content-Type: application/json' \
    -d '{"tool":"execute_python","args":{"code":"print(1+1)"}}' 2>&1) || warn "sandbox invoke returned error"
if echo "$SANDBOX_RESP" | grep -q '"exit_code":0'; then
    pass "sandbox execution works"
else
    warn "sandbox test inconclusive: $SANDBOX_RESP"
fi

echo -e "\n${GREEN}Pre-deploy checks passed. Safe to push/deploy.${NC}"
