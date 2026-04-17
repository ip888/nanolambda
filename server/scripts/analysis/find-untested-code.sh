#!/bin/bash
# Find code that needs test coverage

RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
CYAN='\033[0;36m'
NC='\033[0m'

echo "🔍 Analyzing Code Coverage Gaps"
echo "================================"
echo ""

# Method 1: Find files without test modules
echo "📁 Files Without Test Modules:"
echo "--------------------------------"
for file in $(find crates -name "*.rs" -type f -path "*/src/*" ! -path "*/target/*" ! -name "*test*.rs"); do
    if ! grep -q "#\[cfg(test)\]" "$file" 2>/dev/null; then
        lines=$(wc -l < "$file")
        if [ "$lines" -gt 50 ]; then  # Only show files >50 lines
            echo -e "${RED}❌ $file${NC} (${lines} lines, no tests)"
        fi
    fi
done
echo ""

# Method 2: Find public functions without tests
echo "🔧 Public Functions That May Need Tests:"
echo "----------------------------------------"
for file in $(find crates -name "*.rs" -type f -path "*/src/*" ! -path "*/target/*" ! -name "*test*.rs"); do
    # Count public functions
    pub_fn_count=$(grep -c "pub fn" "$file" 2>/dev/null || echo 0)
    # Count test functions in same file
    test_fn_count=$(grep -c "#\[test\]" "$file" 2>/dev/null || echo 0)
    
    if [ "$pub_fn_count" -gt 0 ]; then
        ratio=0
        if [ "$test_fn_count" -gt 0 ]; then
            ratio=$(echo "scale=1; $test_fn_count * 100 / $pub_fn_count" | bc 2>/dev/null || echo 0)
        fi
        
        if [ "$pub_fn_count" -gt 5 ]; then
            if [ "$test_fn_count" -eq 0 ]; then
                echo -e "${RED}❌ $file${NC}"
                echo "   Public functions: $pub_fn_count | Tests: $test_fn_count (0% coverage)"
            elif [ "${ratio%.*}" -lt 30 ]; then
                echo -e "${YELLOW}⚠️  $file${NC}"
                echo "   Public functions: $pub_fn_count | Tests: $test_fn_count (~${ratio}% coverage)"
            fi
        fi
    fi
done
echo ""

# Method 3: Critical files by name
echo "🚨 Critical Files (Should Have High Coverage):"
echo "----------------------------------------------"
critical_patterns="payment|billing|auth|security|invoice|tier|trial"
for file in $(find crates -name "*.rs" -type f -path "*/src/*" ! -path "*/target/*" | grep -E "$critical_patterns"); do
    has_tests=$(grep -c "#\[test\]" "$file" 2>/dev/null || echo 0)
    lines=$(wc -l < "$file")
    
    if [ "$has_tests" -eq 0 ]; then
        echo -e "${RED}❌ $file${NC} (${lines} lines, ${has_tests} tests)"
    elif [ "$has_tests" -lt 3 ]; then
        echo -e "${YELLOW}⚠️  $file${NC} (${lines} lines, ${has_tests} tests - needs more)"
    else
        echo -e "${GREEN}✅ $file${NC} (${lines} lines, ${has_tests} tests)"
    fi
done
echo ""

# Method 4: Analyze error handling
echo "⚠️  Error Handling Patterns:"
echo "---------------------------"
echo "Files with Result<> but no error tests:"
for file in $(find crates/storage/src crates/api-server/src -name "*.rs" -type f ! -path "*/target/*" ! -name "*test*.rs" | head -10); do
    result_count=$(grep -c "Result<" "$file" 2>/dev/null || echo 0)
    error_tests=$(grep -c "assert.*Err\|unwrap_err\|is_err" "$file" 2>/dev/null || echo 0)
    
    if [ "$result_count" -gt 3 ] && [ "$error_tests" -eq 0 ]; then
        echo -e "${YELLOW}⚠️  $file${NC} - $result_count Result<> types, no error tests"
    fi
done
echo ""

# Method 5: Integration test gaps
echo "🔗 Integration Test Coverage:"
echo "-----------------------------"
if [ -d "crates/storage/tests" ]; then
    storage_tests=$(ls crates/storage/tests/*.rs 2>/dev/null | wc -l)
    echo "Storage integration tests: $storage_tests"
else
    echo -e "${RED}❌ No storage/tests directory${NC}"
fi

if [ -d "crates/api-server/tests" ]; then
    api_tests=$(ls crates/api-server/tests/*.rs 2>/dev/null | wc -l)
    echo "API integration tests: $api_tests"
else
    echo -e "${YELLOW}⚠️  No api-server/tests directory${NC}"
fi

if [ -d "crates/runtime/tests" ]; then
    runtime_tests=$(ls crates/runtime/tests/*.rs 2>/dev/null | wc -l)
    echo "Runtime integration tests: $runtime_tests"
else
    echo -e "${YELLOW}⚠️  No runtime/tests directory${NC}"
fi
echo ""

# Summary recommendations
echo "💡 Recommendations:"
echo "-------------------"
echo "1. Focus on files marked ❌ (no tests)"
echo "2. Add error handling tests for Result<> types"
echo "3. Increase coverage for critical modules (payment, auth, etc.)"
echo "4. Add integration tests for cross-module workflows"
echo ""
echo "📊 To see actual line-by-line coverage:"
echo "   cargo install cargo-llvm-cov"
echo "   cargo llvm-cov --all-features --workspace --html"
echo "   open target/llvm-cov/html/index.html"
echo ""
echo "🎯 To find specific uncovered lines:"
echo "   cargo llvm-cov --all-features --workspace --text | grep -A5 'Uncovered'"
