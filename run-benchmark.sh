#!/bin/bash
# Quick benchmark runner for NanoLambda (local only)

set -e

echo "🚀 NanoLambda Quick Benchmark"
echo ""

# Check if server is running
if ! curl -s http://localhost:3000/health > /dev/null 2>&1; then
    echo "❌ NanoLambda server is not running on port 3000"
    echo ""
    echo "Please start the server first:"
    echo "  cargo run --release --bin server"
    echo ""
    exit 1
fi

echo "✅ Server detected on http://localhost:3000"
echo ""

# Parse arguments
WORKLOAD="${1:-all}"
ITERATIONS="${2:-100}"

if [ "$WORKLOAD" = "all" ]; then
    echo "📊 Running all workloads with $ITERATIONS iterations each..."
    cargo run --release --manifest-path benchmarks/Cargo.toml -- \
        --platform nanolambda \
        --iterations "$ITERATIONS" \
        --output "benchmark_results_$(date +%Y%m%d_%H%M%S).json"
else
    echo "📊 Running '$WORKLOAD' workload with $ITERATIONS iterations..."
    cargo run --release --manifest-path benchmarks/Cargo.toml -- \
        --platform nanolambda \
        --workload-type "$WORKLOAD" \
        --iterations "$ITERATIONS" \
        --output "benchmark_${WORKLOAD}_$(date +%Y%m%d_%H%M%S).json"
fi

echo ""
echo "✅ Benchmark complete!"
