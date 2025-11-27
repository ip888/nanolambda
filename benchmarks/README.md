# NanoLambda vs AWS Lambda Benchmarks

This directory contains benchmark comparisons between NanoLambda and AWS Lambda across various workload types.

## Benchmark Categories

1. **Hello World** - Minimal overhead baseline
2. **JSON Processing** - Realistic API workloads
3. **Compute Heavy** - CPU-intensive calculations
4. **I/O Operations** - File and network I/O patterns

## Running Benchmarks

### Against NanoLambda (Local)
```bash
cargo run --release --bin benchmark-runner -- --platform nanolambda
```

### Against AWS Lambda (Requires AWS credentials)
```bash
export AWS_REGION=us-east-1
cargo run --release --bin benchmark-runner -- --platform aws-lambda
```

### Comparison Mode
```bash
cargo run --release --bin benchmark-runner -- --platform both --output results.json
```

## Metrics Collected

- **Cold Start Time**: First invocation latency
- **Warm Start Time**: Subsequent invocation latency (avg of 100 calls)
- **Throughput**: Requests per second under load
- **Memory Usage**: Peak memory consumption
- **P50/P95/P99**: Latency percentiles
- **Cost**: Estimated cost per million invocations

## Expected Results

Based on our warm start optimization:
- **NanoLambda Cold Start**: ~32ms
- **NanoLambda Warm Start**: ~0-1ms (19x faster than cold)
- **AWS Lambda Cold Start**: 100-300ms (Python 3.11)
- **AWS Lambda Warm Start**: 1-5ms

Our advantage is in:
1. **Extreme warm start performance** (~0ms vs 1-5ms)
2. **Lower cold start overhead** (no container init)
3. **Process pooling efficiency** (instant reuse)
