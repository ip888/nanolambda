#!/bin/bash

# NanoLambda Complete Test Suite Runner
# Runs all test scenarios across all supported languages

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
RESULTS_DIR="$SCRIPT_DIR/results/$(date +%Y%m%d_%H%M%S)"
mkdir -p "$RESULTS_DIR"

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

log() {
    echo -e "${BLUE}[$(date +%H:%M:%S)]${NC} $1"
}

success() {
    echo -e "${GREEN}✓${NC} $1"
}

error() {
    echo -e "${RED}✗${NC} $1"
}

warn() {
    echo -e "${YELLOW}⚠${NC} $1"
}

# Check prerequisites
check_prerequisites() {
    log "Checking prerequisites..."
    
    # Check server
    if ! curl -s http://localhost:8080/health > /dev/null 2>&1; then
        error "NanoLambda server not running on localhost:8080"
        exit 1
    fi
    success "Server is running"
    
    # Check Python
    if ! command -v python3 &> /dev/null; then
        warn "Python3 not found - Python tests will be skipped"
    else
        success "Python $(python3 --version | cut -d' ' -f2) found"
    fi
    
    # Check Node.js
    if ! command -v node &> /dev/null; then
        warn "Node.js not found - Node.js tests will be skipped"
    else
        success "Node.js $(node --version) found"
    fi
    
    # Check Java
    if ! command -v java &> /dev/null; then
        warn "Java not found - Java tests will be skipped"
    else
        success "Java $(java -version 2>&1 | head -1 | cut -d'"' -f2) found"
    fi
}

# Create API key
create_api_key() {
    log "Creating API key..."
    API_KEY=$(curl -s -X POST http://localhost:8080/auth/keys \
        -H "Content-Type: application/json" \
        -d '{"name": "test-suite-'$(date +%s)'", "expires_at": null}' | \
        grep -o '"key":"[^"]*' | cut -d'"' -f4)
    
    if [ -z "$API_KEY" ]; then
        error "Failed to create API key"
        exit 1
    fi
    
    echo "$API_KEY" > "$RESULTS_DIR/api_key.txt"
    export API_KEY
    success "API key created: ${API_KEY:0:20}..."
}

# Run test scenario
run_scenario() {
    local language=$1
    local scenario=$2
    local description=$3
    
    log "Running: $description"
    
    SCENARIO_RESULTS="$RESULTS_DIR/${language}_${scenario}"
    mkdir -p "$SCENARIO_RESULTS"
    
    bash "$SCRIPT_DIR/run-tests.sh" "$language" "$scenario" > "$SCENARIO_RESULTS/output.log" 2>&1
    
    if [ $? -eq 0 ]; then
        success "$description completed"
        return 0
    else
        error "$description failed"
        return 1
    fi
}

# Generate summary report
generate_report() {
    log "Generating test report..."
    
    REPORT="$RESULTS_DIR/summary.md"
    
    cat > "$REPORT" << EOF
# NanoLambda Test Suite Results

**Date:** $(date)
**Duration:** $(($(date +%s) - START_TIME)) seconds

## Test Summary

EOF
    
    # Count results
    local total=0
    local passed=0
    local failed=0
    
    for log in "$RESULTS_DIR"/*/output.log; do
        total=$((total + 1))
        if grep -q "✓" "$log"; then
            passed=$((passed + 1))
        else
            failed=$((failed + 1))
        fi
    done
    
    cat >> "$REPORT" << EOF
- **Total Tests:** $total
- **Passed:** $passed
- **Failed:** $failed
- **Success Rate:** $(awk "BEGIN {printf \"%.1f\", ($passed/$total)*100}")%

## Detailed Results

EOF
    
    for dir in "$RESULTS_DIR"/*/; do
        if [ -f "$dir/output.log" ]; then
            scenario=$(basename "$dir")
            echo "### $scenario" >> "$REPORT"
            echo '```' >> "$REPORT"
            tail -20 "$dir/output.log" >> "$REPORT"
            echo '```' >> "$REPORT"
            echo "" >> "$REPORT"
        fi
    done
    
    success "Report saved to: $REPORT"
    
    # Create symlink to latest results
    rm -f "$SCRIPT_DIR/results/latest"
    ln -s "$RESULTS_DIR" "$SCRIPT_DIR/results/latest"
}

# Main execution
main() {
    echo "╔════════════════════════════════════════════════════════════╗"
    echo "║        NanoLambda Complete Test Suite                     ║"
    echo "╚════════════════════════════════════════════════════════════╝"
    echo ""
    
    START_TIME=$(date +%s)
    
    check_prerequisites
    create_api_key
    
    echo ""
    log "Starting test execution..."
    echo ""
    
    # Phase 1: Basic Sanity Tests (all languages)
    echo "═══════════════════════════════════════════════════════════"
    echo "PHASE 1: Basic Sanity Tests"
    echo "═══════════════════════════════════════════════════════════"
    
    run_scenario "python" "sanity" "Python sanity test"
    run_scenario "nodejs" "sanity" "Node.js sanity test"
    run_scenario "java" "sanity" "Java sanity test"
    
    # Phase 2: Production Load Tests
    echo ""
    echo "═══════════════════════════════════════════════════════════"
    echo "PHASE 2: Production Load Tests"
    echo "═══════════════════════════════════════════════════════════"
    
    run_scenario "python" "load" "Python production load"
    run_scenario "nodejs" "load" "Node.js production load"
    run_scenario "java" "load" "Java production load"
    
    # Phase 3: Cross-Language Integration
    echo ""
    echo "═══════════════════════════════════════════════════════════"
    echo "PHASE 3: Cross-Language Integration"
    echo "═══════════════════════════════════════════════════════════"
    
    run_scenario "mixed" "integration" "Cross-language integration"
    
    # Phase 4: Stress Test
    echo ""
    echo "═══════════════════════════════════════════════════════════"
    echo "PHASE 4: Stress Test"
    echo "═══════════════════════════════════════════════════════════"
    
    bash "$SCRIPT_DIR/stress-test.sh" > "$RESULTS_DIR/stress_test.log" 2>&1
    
    echo ""
    generate_report
    
    END_TIME=$(date +%s)
    DURATION=$((END_TIME - START_TIME))
    
    echo ""
    echo "╔════════════════════════════════════════════════════════════╗"
    echo "║                  Test Suite Complete                       ║"
    echo "╚════════════════════════════════════════════════════════════╝"
    echo ""
    success "Total duration: $(($DURATION / 60))m $(($DURATION % 60))s"
    echo ""
    echo "View results:"
    echo "  - Report: $REPORT"
    echo "  - Dashboard: http://localhost:8080/dashboard"
    echo "  - Results: $RESULTS_DIR"
    echo ""
}

main "$@"
