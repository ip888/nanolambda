# Test Coverage Guide for Production Readiness

## Current Test Status

**Test Count**: ~50+ unit tests + integration tests
- ✅ Unit tests across all major modules
- ✅ Integration tests for storage, runtime, API
- ⚠️ 3 Python runtime tests currently failing (needs fixing)

## Production-Ready Coverage Standards

### Industry Benchmarks

```
Minimum Standards:
- Critical paths: 90-100% coverage
- Business logic: 80-90% coverage
- Overall codebase: 70-80% coverage
- Public APIs: 95-100% coverage

Rust Best Practices:
- Core libraries: 80%+ overall
- Web APIs: 75%+ overall
- Infrastructure: 70%+ overall
```

### Coverage by Module Type

```
1. Storage Layer (Critical): Target 85%+
   - Database operations
   - Data integrity
   - Transaction handling

2. Runtime/Execution (Critical): Target 80%+
   - Function execution
   - Memory management
   - Error handling

3. API Handlers (High): Target 75%+
   - Request handling
   - Authentication
   - Rate limiting

4. Utilities (Medium): Target 70%+
   - Helper functions
   - Formatters
   - Validators
```

## Measuring Coverage

### Option 1: cargo-tarpaulin (Recommended for Linux)

```bash
# Install
cargo install cargo-tarpaulin

# Run coverage (all features)
cargo tarpaulin --all-features --workspace --timeout 300 --out Html

# With minimum threshold enforcement
cargo tarpaulin --all-features --workspace --fail-under 70

# Generate multiple report formats
cargo tarpaulin --all-features --workspace --out Html --out Json --out Lcov

# Exclude specific paths
cargo tarpaulin --exclude-files 'target/*' --exclude-files 'tests/*'

# View report
open tarpaulin-report.html
```

**Pros**: Native Rust tool, accurate, generates HTML reports
**Cons**: Linux only (uses ptrace)

### Option 2: cargo-llvm-cov (Cross-platform)

```bash
# Install
cargo install cargo-llvm-cov

# Run coverage
cargo llvm-cov --all-features --workspace --html

# With threshold
cargo llvm-cov --all-features --workspace --fail-under-lines 70

# Generate lcov format (for CI tools)
cargo llvm-cov --all-features --workspace --lcov --output-path lcov.info

# View report
open target/llvm-cov/html/index.html
```

**Pros**: Cross-platform, uses LLVM instrumentation
**Cons**: Requires rust nightly or specific setup

### Option 3: kcov (Generic Linux tool)

```bash
# Install (Ubuntu/Debian)
sudo apt-get install kcov

# Build tests with debug info
cargo test --no-run

# Run coverage
for file in target/debug/deps/*-*; do 
    if [[ -x "$file" ]]; then
        mkdir -p "target/cov/$(basename $file)"
        kcov --exclude-pattern=/.cargo,/usr/lib --verify "target/cov/$(basename $file)" "$file"
    fi
done

# Merge reports
kcov --merge target/cov/merged target/cov/*

# View report
open target/cov/merged/index.html
```

### Option 4: Codecov.io / Coveralls (CI Integration)

```yaml
# .github/workflows/coverage.yml
name: Coverage

on: [push, pull_request]

jobs:
  coverage:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v3
      
      - name: Install Rust
        uses: actions-rs/toolchain@v1
        with:
          toolchain: stable
          
      - name: Install tarpaulin
        run: cargo install cargo-tarpaulin
        
      - name: Generate coverage
        run: cargo tarpaulin --all-features --workspace --timeout 300 --out Xml
        
      - name: Upload to codecov.io
        uses: codecov/codecov-action@v3
        with:
          files: ./cobertura.xml
          fail_ci_if_error: true
```

## What to Test (Priority Order)

### 1. Critical Paths (Must Have 90%+)

```rust
✓ Storage operations (CRUD)
✓ Function execution
✓ Authentication & authorization
✓ Payment processing
✓ Data integrity checks
✓ Error handling for critical operations
```

### 2. Business Logic (Must Have 80%+)

```rust
✓ Billing calculations
✓ Rate limiting
✓ Tier management
✓ Usage tracking
✓ API key management
✓ Concurrency control
```

### 3. Edge Cases (Should Have 70%+)

```rust
✓ Invalid inputs
✓ Boundary conditions
✓ Race conditions
✓ Timeout scenarios
✓ Resource exhaustion
✓ Network failures
```

### 4. Integration Points (Should Have 75%+)

```rust
✓ Database connections
✓ External API calls (Stripe)
✓ File I/O operations
✓ Process spawning
✓ Inter-module communication
```

## Coverage Analysis

### Current nanolambda Coverage Estimate

Based on test count and code structure:

```
Module                    Tests  Estimated Coverage  Target
─────────────────────────────────────────────────────────────
runtime/executor          4      ~60%               80%
runtime/nodejs            7      ~75%               80%
runtime/pool              2      ~50%               75%
runtime/metrics           6      ~80%               75%
runtime/types             3      ~70%               70%
storage/manager           7      ~65%               85%
api-server/auth           3      ~80%               75%
api-server/rate_limiter   2      ~70%               75%
api-server/concurrency    2      ~65%               75%
api-server/metrics        5      ~75%               75%

Integration Tests:
- versioning              2      Good
- api_keys               7      Good
- warm_start             3      Good
─────────────────────────────────────────────────────────────
ESTIMATED OVERALL:               ~68%               75%+
```

## Improving Coverage

### Priority 1: Fix Failing Tests
```bash
# These must pass for accurate coverage
- executor::tests::test_simple_function
- executor::tests::test_function_with_event
- pool::tests::test_warm_execution
```

### Priority 2: Add Missing Critical Tests

```rust
// Storage layer gaps
#[test]
fn test_concurrent_database_access() {}

#[test]
fn test_transaction_rollback() {}

#[test]
fn test_database_connection_pool_exhaustion() {}

// Runtime gaps
#[test]
fn test_memory_limit_enforcement() {}

#[test]
fn test_timeout_enforcement() {}

#[test]
fn test_concurrent_execution_limits() {}

// API gaps
#[test]
fn test_rate_limit_distributed_scenario() {}

#[test]
fn test_authentication_token_expiry() {}

#[test]
fn test_request_payload_size_limits() {}
```

### Priority 3: Integration Tests

```rust
// End-to-end scenarios
#[test]
fn test_full_function_lifecycle() {}

#[test]
fn test_billing_workflow() {}

#[test]
fn test_tier_upgrade_downgrade() {}

#[test]
fn test_payment_failure_handling() {}
```

## Running Coverage Analysis

### Quick Coverage Check Script

```bash
#!/bin/bash
# coverage-check.sh

echo "🔍 Analyzing test coverage..."

# Method 1: Simple line counting (rough estimate)
echo -e "\n📊 Test Statistics:"
echo "Total test functions: $(grep -r "#\[test\]" --include="*.rs" . | wc -l)"
echo "Test modules: $(grep -r "#\[cfg(test)\]" --include="*.rs" . | wc -l)"

# Method 2: Run tests with verbose output
echo -e "\n🧪 Running tests..."
cargo test --all-features --workspace 2>&1 | tee test-output.txt

# Count test results
PASSED=$(grep -o "test result.*passed" test-output.txt | grep -o "[0-9]* passed" | awk '{print $1}')
FAILED=$(grep -o "test result.*failed" test-output.txt | grep -o "[0-9]* failed" | awk '{print $1}')

echo -e "\n📈 Test Results:"
echo "Passed: $PASSED"
echo "Failed: $FAILED"

# Method 3: Estimate coverage from test-to-code ratio
TOTAL_LINES=$(find . -name "*.rs" -not -path "*/target/*" -not -path "*/tests/*" | xargs wc -l | tail -1 | awk '{print $1}')
TEST_LINES=$(find . -name "*.rs" -path "*/tests/*" -o -name "*.rs" -exec grep -l "#\[cfg(test)\]" {} \; | xargs wc -l 2>/dev/null | tail -1 | awk '{print $1}')

echo -e "\n📏 Code Metrics:"
echo "Production code lines: ~$TOTAL_LINES"
echo "Test code lines: ~$TEST_LINES"
echo "Test-to-code ratio: ~$(echo "scale=2; $TEST_LINES * 100 / $TOTAL_LINES" | bc)%"

# Recommendation
if [ $FAILED -gt 0 ]; then
    echo -e "\n⚠️  FIX FAILING TESTS FIRST"
fi

echo -e "\n💡 For accurate coverage, install and run:"
echo "   cargo install cargo-llvm-cov"
echo "   cargo llvm-cov --all-features --workspace --html"
```

### CI/CD Coverage Enforcement

```yaml
# Add to .github/workflows/test.yml
- name: Install coverage tool
  run: cargo install cargo-llvm-cov

- name: Run coverage
  run: |
    cargo llvm-cov --all-features --workspace --lcov --output-path lcov.info
    cargo llvm-cov report --fail-under-lines 70

- name: Upload coverage
  uses: codecov/codecov-action@v3
  with:
    files: ./lcov.info
```

## Production Readiness Checklist

### Coverage Requirements

- [ ] **Overall coverage ≥ 70%**
- [ ] **Critical modules ≥ 80%**
- [ ] **Public APIs ≥ 90%**
- [ ] **All tests passing**
- [ ] **No flaky tests**
- [ ] **Integration tests for key workflows**
- [ ] **Error paths covered**
- [ ] **Edge cases tested**

### Additional Quality Gates

- [ ] **Benchmark tests for performance**
- [ ] **Load/stress tests for API**
- [ ] **Security tests (injection, auth bypass)**
- [ ] **Chaos testing for resilience**
- [ ] **Documentation tests (doc examples)**

## Recommended Commands

```bash
# 1. Run all tests
cargo test --all-features --workspace --verbose

# 2. Run with coverage (choose one):
cargo llvm-cov --all-features --workspace --html           # Cross-platform
cargo tarpaulin --all-features --workspace --out Html      # Linux only

# 3. Check coverage threshold
cargo llvm-cov --all-features --workspace --fail-under-lines 70

# 4. Generate coverage for CI
cargo llvm-cov --all-features --workspace --lcov --output-path lcov.info

# 5. View detailed test output
cargo test --all-features --workspace -- --nocapture

# 6. Run specific test with coverage
cargo llvm-cov --test integration_test --html
```

## Best Practices

### 1. Write Testable Code

```rust
// ✅ Good: Easy to test
pub fn calculate_price(units: u64, rate: f64) -> f64 {
    units as f64 * rate
}

// ❌ Bad: Hard to test (hidden dependencies)
pub fn calculate_price() -> f64 {
    let units = DATABASE.query("SELECT units");
    let rate = CONFIG.get_rate();
    units * rate
}
```

### 2. Use Test Fixtures

```rust
#[cfg(test)]
mod tests {
    use super::*;
    
    fn setup_test_db() -> StorageManager {
        StorageManager::new(":memory:").unwrap()
    }
    
    #[test]
    fn test_with_fixture() {
        let db = setup_test_db();
        // test with db
    }
}
```

### 3. Test Error Cases

```rust
#[test]
fn test_invalid_input() {
    let result = function_under_test(invalid_input);
    assert!(result.is_err());
    assert_eq!(result.unwrap_err().to_string(), "Expected error");
}
```

### 4. Use Property-Based Testing

```rust
// Add to Cargo.toml: proptest = "1.0"
use proptest::prelude::*;

proptest! {
    #[test]
    fn test_reversible(s: String) {
        assert_eq!(reverse(&reverse(&s)), s);
    }
}
```

## Summary: nanolambda Production Readiness

**Current State:**
- ✅ Good test foundation (~50+ tests)
- ⚠️ Estimated 68% coverage (needs verification)
- ❌ 3 failing tests (must fix)
- ⚠️ No automated coverage tracking

**To Achieve Production Readiness:**

1. **Fix failing tests** (Priority 1)
2. **Measure actual coverage** with cargo-llvm-cov
3. **Add tests to reach 75%+ overall**
4. **Focus on critical paths** (80%+ for storage, runtime)
5. **Set up CI coverage reporting**
6. **Add integration tests** for key workflows
7. **Document testing strategy**

**Estimated Effort:**
- Fix failing tests: 1-2 hours
- Measure coverage: 30 minutes
- Reach 75% coverage: 4-8 hours
- Set up CI: 1 hour
- Total: 1-2 days of focused work

Your codebase has a solid testing foundation. Adding coverage measurement and filling gaps will get you to production-ready status quickly!
