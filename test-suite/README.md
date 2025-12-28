# NanoLambda Test Suite

## Overview
Production-grade test suite with automated scripts, multi-language function examples, and continuous monitoring.

## Quick Start

```bash
# Run complete test suite (all languages, all scenarios)
./test-suite/run-all-tests.sh

# Run specific language tests
./test-suite/run-tests.sh python
./test-suite/run-tests.sh nodejs
./test-suite/run-tests.sh java

# Run high-load stress test
./test-suite/stress-test.sh

# Start 24/7 continuous testing
./test-suite/continuous-monitor.sh start

# View test results
./test-suite/show-results.sh
```

## Directory Structure

```
test-suite/
├── README.md                    # This file
├── run-all-tests.sh            # Master test runner
├── run-tests.sh                # Individual language test runner
├── stress-test.sh              # High-load testing
├── continuous-monitor.sh       # 24/7 monitoring
├── show-results.sh             # Results viewer
├── cleanup.sh                  # Clean test data
├── functions/                  # Pre-built function sets
│   ├── python/                 # Python examples
│   │   ├── rest-api/
│   │   ├── data-processing/
│   │   ├── ml-inference/
│   │   └── websocket-handler/
│   ├── nodejs/                 # Node.js examples
│   │   ├── express-api/
│   │   ├── image-processor/
│   │   ├── real-time-chat/
│   │   └── stream-processor/
│   └── java/                   # Java examples
│       ├── spring-boot-api/
│       ├── kafka-consumer/
│       ├── batch-processor/
│       └── microservice/
├── scenarios/                  # Test scenarios
│   ├── basic-sanity.json
│   ├── production-load.json
│   ├── stress-test.json
│   └── continuous-monitoring.json
└── results/                    # Test results (auto-generated)
    ├── latest/
    ├── history/
    └── reports/
```

## Test Scenarios

### 1. Basic Sanity (5 minutes)
- Deploy 1 function per language
- Test cold/warm starts
- Verify metrics accuracy

### 2. Production Load (30 minutes)
- Deploy 10 functions across languages
- 1000 req/min sustained load
- Monitor latency, errors, memory

### 3. Stress Test (1 hour)
- 50 concurrent functions
- 10,000 req/min peak load
- Resource exhaustion testing
- Recovery validation

### 4. Continuous Monitoring (24/7)
- Rotating function deployments
- Variable load patterns
- Automatic health checks
- Alert on anomalies

## Viewing Results

Dashboard: http://localhost:8080/dashboard
- Real-time metrics
- No sample data
- Live invocation counts
- Actual latency distributions

Test Reports: `./test-suite/results/latest/`
- Summary statistics
- Performance graphs
- Error logs
- Resource usage

## Platform Support

All scripts support:
- Linux (bash)
- macOS (bash/zsh)
- Windows (PowerShell via WSL or Git Bash)

## Requirements

- NanoLambda server running
- Python 3.8+ installed
- Node.js 14+ installed
- Java 11+ installed (for Java tests)
- curl or PowerShell
- jq (optional, for JSON parsing)
