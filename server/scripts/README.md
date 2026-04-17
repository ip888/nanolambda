# Scripts Directory

## Overview

Organized shell scripts for testing, utilities, and benchmarks.

## Directory Structure

```
scripts/
├── testing/          Test and verification scripts
├── utilities/        General utility scripts
├── benchmarks/       Performance benchmark scripts
└── analysis/         Code analysis scripts
```

## Testing Scripts (`testing/`)

- `test-platform.sh` - Verify platform compatibility
- `test-webhooks.sh` - Test webhook functionality  
- `test_dashboard.sh` - Dashboard integration tests
- `test_dashboard_working.sh` - Dashboard verification

**Usage:**
```bash
./scripts/testing/test-platform.sh
```

## Utility Scripts (`utilities/`)

- `demo.sh` - Project demonstration
- `quick_test.sh` - Quick sanity checks
- `quality-check.sh` - Code quality verification
- `cleanup-repo.sh` - Repository cleanup (git-tracked files)
- `cleanup-docs.sh` - Documentation cleanup

**Usage:**
```bash
./scripts/utilities/demo.sh
./scripts/utilities/quality-check.sh
```

## Benchmark Scripts (`benchmarks/`)

- `run-benchmark.sh` - Performance benchmarks

**Usage:**
```bash
./scripts/benchmarks/run-benchmark.sh
```

## Analysis Scripts (`analysis/`)

Code analysis and metrics collection.

---

## Note on test-suite/

The `test-suite/` directory at project root contains the comprehensive test infrastructure:
- `test-suite/run-all-tests.sh` - Main test suite runner
- `test-suite/run-tests.sh` - Per-language tests
- `test-suite/stress-test.sh` - Load testing
- `test-suite/continuous-monitor.sh` - CI/CD monitoring

These are separate from the individual test scripts in `scripts/testing/`.
