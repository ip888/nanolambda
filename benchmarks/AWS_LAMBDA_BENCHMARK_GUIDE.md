# AWS Lambda Benchmark Guide

This guide explains how to run benchmarks comparing NanoLambda with AWS Lambda.

## Prerequisites

### For NanoLambda Benchmarks
- NanoLambda server running locally on port 3000
- Python 3.11 or 3.12 installed

### For AWS Lambda Benchmarks
- AWS account with Lambda access
- AWS CLI configured with credentials
- IAM role ARN for Lambda execution

## Quick Start

### 1. Start NanoLambda Server
```bash
# Terminal 1: Start the server
cargo run --release --bin server
```

### 2. Run NanoLambda Benchmarks Only
```bash
# Terminal 2: Run benchmarks
cd benchmarks
cargo run --release -- --platform nanolambda
```

### 3. Run AWS Lambda Benchmarks (Optional)
```bash
# Set required environment variables
export AWS_LAMBDA_ROLE_ARN="arn:aws:iam::YOUR_ACCOUNT:role/lambda-execution-role"
export AWS_REGION="us-east-1"

# Run AWS benchmarks
cargo run --release -- --platform aws-lambda
```

### 4. Run Comparison Benchmarks
```bash
# Compare both platforms
cargo run --release -- --platform both --output results.json
```

## Command-Line Options

```
benchmark-runner [OPTIONS]

Options:
  -p, --platform <PLATFORM>
          Platform to benchmark: nanolambda, aws-lambda, or both
          [default: nanolambda]

  -w, --warmup <WARMUP>
          Number of warm-up invocations
          [default: 10]

  -i, --iterations <ITERATIONS>
          Number of benchmark iterations
          [default: 100]

  -o, --output <OUTPUT>
          Output file for results (JSON)

  -t, --workload-type <WORKLOAD_TYPE>
          Specific workload to run (default: all)
          Options: hello, json, compute, io

  -h, --help
          Print help

  -V, --version
          Print version
```

## Workload Types

### 1. Hello World
- **Purpose**: Baseline minimal overhead test
- **Operations**: Simple string formatting
- **Expected Time**: < 1ms warm start

### 2. JSON Processing
- **Purpose**: Realistic API workload
- **Operations**: Parse, filter, transform, aggregate JSON data
- **Expected Time**: 1-3ms warm start

### 3. Compute Heavy
- **Purpose**: CPU-intensive calculations
- **Operations**: Math operations, prime checks, trigonometry
- **Expected Time**: 10-50ms depending on iterations

### 4. I/O Operations
- **Purpose**: File system operations
- **Operations**: Create temp files, write, read, process
- **Expected Time**: 5-15ms

## Example Runs

### Test specific workload
```bash
cargo run --release -- --platform nanolambda --workload-type json
```

### High iteration count for accuracy
```bash
cargo run --release -- --platform nanolambda --iterations 1000
```

### Save results for analysis
```bash
cargo run --release -- --platform both --output comparison_$(date +%Y%m%d).json
```

## Understanding Results

### Metrics Explained

- **Cold Start**: First invocation after deployment (includes initialization)
- **Warm P50**: Median latency for warmed-up invocations
- **Warm P95**: 95th percentile latency
- **Warm P99**: 99th percentile latency
- **Throughput**: Requests per second under sustained load
- **Memory**: Peak memory usage in MB

### Expected NanoLambda Performance

Based on our warm start optimization:

```
Workload          Cold Start    Warm P50    Throughput
Hello World       ~25-35ms      ~0-1ms      500+ req/s
JSON Processing   ~30-40ms      ~1-2ms      400+ req/s
Compute Heavy     ~35-45ms      ~10-20ms    50-100 req/s
I/O Operations    ~30-40ms      ~5-10ms     100-200 req/s
```

### AWS Lambda Comparison

Typical AWS Lambda (Python 3.11):

```
Workload          Cold Start    Warm P50    Throughput
Hello World       ~100-200ms    ~1-5ms      200-400 req/s
JSON Processing   ~150-250ms    ~3-8ms      150-300 req/s
Compute Heavy     ~150-250ms    ~15-30ms    40-80 req/s
I/O Operations    ~150-250ms    ~10-20ms    80-150 req/s
```

### Speedup Analysis

Expected advantages:

- **Cold Start**: 3-7x faster (no container overhead)
- **Warm Start**: 2-10x faster (process pooling)
- **Throughput**: 1.5-2.5x higher (lower latency)

## AWS Lambda Setup

### Creating Lambda Execution Role

```bash
# Create trust policy
cat > trust-policy.json << EOF
{
  "Version": "2012-10-17",
  "Statement": [{
    "Effect": "Allow",
    "Principal": {"Service": "lambda.amazonaws.com"},
    "Action": "sts:AssumeRole"
  }]
}
EOF

# Create role
aws iam create-role \
  --role-name lambda-benchmark-role \
  --assume-role-policy-document file://trust-policy.json

# Attach basic execution policy
aws iam attach-role-policy \
  --role-name lambda-benchmark-role \
  --policy-arn arn:aws:iam::aws:policy/service-role/AWSLambdaBasicExecutionRole

# Get role ARN
aws iam get-role --role-name lambda-benchmark-role --query 'Role.Arn' --output text
```

### Set Environment Variable

```bash
export AWS_LAMBDA_ROLE_ARN=$(aws iam get-role --role-name lambda-benchmark-role --query 'Role.Arn' --output text)
```

## Troubleshooting

### NanoLambda server not responding
```bash
# Check if server is running
curl http://localhost:3000/health

# Check server logs
cargo run --release --bin server
```

### AWS Lambda permission errors
```bash
# Verify AWS credentials
aws sts get-caller-identity

# Check role exists
aws iam get-role --role-name lambda-benchmark-role
```

### Benchmark fails with timeout
```bash
# Increase timeout in workload code
# Or reduce iterations for compute-heavy workloads
cargo run --release -- --workload-type hello --iterations 50
```

## Cost Estimates

### NanoLambda
- **Infrastructure**: Free (self-hosted)
- **Development**: Open source

### AWS Lambda (for benchmarking)
- **Invocations**: ~$0.20 per 1M requests
- **Compute**: ~$0.0000166667 per GB-second
- **Benchmark Cost**: < $0.01 for typical test run

**Note**: Clean up AWS resources after benchmarking to avoid charges:
```bash
# Functions are automatically deleted after each benchmark
# But verify with:
aws lambda list-functions --query 'Functions[?starts_with(FunctionName, `bench-`)]'
```

## Next Steps

After running benchmarks:

1. **Analyze Results**: Review the comparison table for speedup factors
2. **Document Findings**: Add results to project documentation
3. **Optimize Further**: Identify bottlenecks from P99 latencies
4. **Share Results**: Include in presentations, README, blog posts
5. **Continuous Monitoring**: Re-run benchmarks after major changes
