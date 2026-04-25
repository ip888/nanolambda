#!/usr/bin/env bash
# =============================================================================
# Production demo walkthrough for NanoLambda
#
# Runs deterministic checks against a deployed instance.
# Usage:
#   BASE_URL="https://your-instance.example.com" API_KEY="nl_..." bash scripts/demo-production.sh
#
# API_KEY is optional for deployments that expose sandbox endpoints publicly,
# but recommended for production environments.
# =============================================================================
set -euo pipefail

GREEN='\033[0;32m'
RED='\033[0;31m'
YELLOW='\033[1;33m'
NC='\033[0m'

pass() { echo -e "${GREEN}PASS${NC} $1"; }
fail() { echo -e "${RED}FAIL${NC} $1"; exit 1; }
warn() { echo -e "${YELLOW}WARN${NC} $1"; }

BASE_URL="${BASE_URL:-}"
API_KEY="${API_KEY:-}"

if [[ -z "$BASE_URL" ]]; then
    fail "BASE_URL is required. Example: BASE_URL=https://your-instance.example.com"
fi

BASE_URL="${BASE_URL%/}"
AUTH_HEADER=()
if [[ -n "$API_KEY" ]]; then
    AUTH_HEADER=(-H "Authorization: Bearer $API_KEY")
else
    warn "API_KEY is not set. Continuing without Authorization header."
fi

echo "Running NanoLambda production demo against: $BASE_URL"

echo "Step 1/4: Health check"
echo "Waiting for service startup (up to 30s)..."
for i in $(seq 1 30); do
    HEALTH_CODE=$(curl -sS -o /tmp/nl_health.json -w '%{http_code}' "$BASE_URL/health" || true)
    if [[ "$HEALTH_CODE" == "200" ]]; then
        pass "/health returned 200"
        break
    fi
    if [[ "$i" -eq 30 ]]; then
        fail "/health did not become ready within 30s (last HTTP code: ${HEALTH_CODE:-none})"
    fi
    sleep 1
done

echo "Step 2/4: Metrics endpoint"
METRICS_CODE=$(curl -sS -o /tmp/nl_metrics.txt -w '%{http_code}' "$BASE_URL/metrics/prometheus")
[[ "$METRICS_CODE" == "200" ]] || fail "/metrics/prometheus returned HTTP $METRICS_CODE"
grep -q "nanolambda_invocations_total" /tmp/nl_metrics.txt || fail "metrics output missing nanolambda_invocations_total"
pass "/metrics/prometheus returned valid metrics"

echo "Step 3/4: Deterministic Python sandbox run"
PAYLOAD='{"tool":"execute_python","args":{"code":"print(2+2)"}}'
SANDBOX_CODE=$(curl -sS -o /tmp/nl_sandbox_1.json -w '%{http_code}' \
    -X POST "$BASE_URL/sandbox/invoke" \
    "${AUTH_HEADER[@]}" \
    -H "Content-Type: application/json" \
    -d "$PAYLOAD")

if [[ "$SANDBOX_CODE" == "401" || "$SANDBOX_CODE" == "403" ]]; then
    fail "sandbox endpoint rejected auth (HTTP $SANDBOX_CODE). Set a valid API_KEY and retry."
fi
[[ "$SANDBOX_CODE" == "200" ]] || fail "/sandbox/invoke returned HTTP $SANDBOX_CODE"
grep -q '"exit_code"[[:space:]]*:[[:space:]]*0' /tmp/nl_sandbox_1.json || fail "sandbox result exit_code != 0"
grep -q '"stdout"' /tmp/nl_sandbox_1.json || fail "sandbox result missing stdout"
grep -q '4' /tmp/nl_sandbox_1.json || fail "sandbox result did not include expected output 4"
pass "sandbox run returned expected output"

echo "Step 4/4: Deterministic data transform demo"
PAYLOAD='{"tool":"execute_python","args":{"code":"nums=[10,20,30]; print(sum(nums))"}}'
SANDBOX_CODE=$(curl -sS -o /tmp/nl_sandbox_2.json -w '%{http_code}' \
    -X POST "$BASE_URL/sandbox/invoke" \
    "${AUTH_HEADER[@]}" \
    -H "Content-Type: application/json" \
    -d "$PAYLOAD")
[[ "$SANDBOX_CODE" == "200" ]] || fail "second sandbox run returned HTTP $SANDBOX_CODE"
grep -q '"exit_code"[[:space:]]*:[[:space:]]*0' /tmp/nl_sandbox_2.json || fail "second sandbox result exit_code != 0"
grep -q '60' /tmp/nl_sandbox_2.json || fail "second sandbox result did not include expected output 60"
pass "second sandbox run returned expected output"

echo
echo -e "${GREEN}Demo completed successfully.${NC}"
echo "Artifacts: /tmp/nl_health.json /tmp/nl_metrics.txt /tmp/nl_sandbox_1.json /tmp/nl_sandbox_2.json"
