#!/bin/bash
# Quick test coverage analysis for nanolambda

set -e

RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m'

echo "🔍 Nanolambda Test Coverage Analysis"
echo "===================================="
echo ""

# Count tests
echo "📊 Test Statistics:"
TEST_FUNCTIONS=$(grep -r "#\[test\]" --include="*.rs" . 2>/dev/null | wc -l)
TEST_MODULES=$(grep -r "#\[cfg(test)\]" --include="*.rs" . 2>/dev/null | wc -l)
echo "  Test functions: $TEST_FUNCTIONS"
echo "  Test modules: $TEST_MODULES"
echo ""

# Run tests
echo "🧪 Running all tests..."
cargo test --all-features --workspace --lib 2>&1 | tee /tmp/test-output.txt

# Parse results
echo ""
echo "📈 Test Results Summary:"
PASSED=$(grep "test result:" /tmp/test-output.txt | tail -1 | grep -o "[0-9]* passed" | awk '{print $1}' || echo "0")
FAILED=$(grep "test result:" /tmp/test-output.txt | tail -1 | grep -o "[0-9]* failed" | awk '{print $1}' || echo "0")

echo -e "  ${GREEN}Passed: $PASSED${NC}"
if [ "$FAILED" != "0" ]; then
    echo -e "  ${RED}Failed: $FAILED${NC}"
fi
echo ""

# Estimate coverage from code metrics
echo "📏 Code Metrics:"
TOTAL_LINES=$(find . -name "*.rs" -not -path "*/target/*" -not -path "*/tests/*" -not -name "*test*.rs" 2>/dev/null | xargs wc -l 2>/dev/null | tail -1 | awk '{print $1}' || echo "0")
TEST_LINES=$(find . -name "*.rs" \( -path "*/tests/*" -o -name "*test*.rs" \) 2>/dev/null | xargs wc -l 2>/dev/null | tail -1 | awk '{print $1}' || echo "0")

echo "  Production code: ~$TOTAL_LINES lines"
echo "  Test code: ~$TEST_LINES lines"

if [ "$TOTAL_LINES" != "0" ]; then
    RATIO=$(echo "scale=1; $TEST_LINES * 100 / $TOTAL_LINES" | bc 2>/dev/null || echo "0")
    echo "  Test-to-code ratio: ~${RATIO}%"
fi
echo ""

# Coverage estimate
echo "📉 Coverage Estimate (based on test-to-code ratio):"
if [ "$TOTAL_LINES" != "0" ]; then
    EST_COVERAGE=$(echo "scale=0; $RATIO * 0.7" | bc 2>/dev/null || echo "0")
    echo "  Estimated coverage: ~${EST_COVERAGE}% (rough approximation)"
fi
echo ""

# Recommendations
echo "💡 Recommendations:"
if [ "$FAILED" != "0" ]; then
    echo -e "  ${RED}⚠️  FIX $FAILED FAILING TEST(S) FIRST${NC}"
fi

echo "  For accurate coverage measurement:"
echo ""
echo "  Option 1 - cargo-llvm-cov (recommended, cross-platform):"
echo "    $ cargo install cargo-llvm-cov"
echo "    $ cargo llvm-cov --all-features --workspace --html"
echo "    $ open target/llvm-cov/html/index.html"
echo ""
echo "  Option 2 - cargo-tarpaulin (Linux only, more accurate):"
echo "    $ cargo install cargo-tarpaulin"
echo "    $ cargo tarpaulin --all-features --workspace --out Html"
echo "    $ open tarpaulin-report.html"
echo ""

# Production readiness check
echo "🎯 Production Readiness:"
if [ "$FAILED" == "0" ] && [ "$PASSED" -gt "40" ]; then
    echo -e "  ${GREEN}✅ Good test foundation ($PASSED passing tests)${NC}"
else
    echo -e "  ${YELLOW}⚠️  Needs more tests or fix failures${NC}"
fi

if [ "$EST_COVERAGE" -ge "70" ] 2>/dev/null; then
    echo -e "  ${GREEN}✅ Estimated coverage appears adequate${NC}"
elif [ "$EST_COVERAGE" -ge "60" ] 2>/dev/null; then
    echo -e "  ${YELLOW}⚠️  Coverage likely 60-70%, aim for 75%+${NC}"
else
    echo -e "  ${RED}❌ Coverage likely <60%, need more tests${NC}"
fi

echo ""
echo "See TEST_COVERAGE_GUIDE.md for detailed coverage standards"
