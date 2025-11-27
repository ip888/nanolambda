use clap::Parser;
use colored::Colorize;
use serde::{Deserialize, Serialize};
use std::time::{Duration, Instant};
use tabled::{Table, Tabled};

mod workloads;
mod platforms;
mod statistics;

use workloads::{Workload, WorkloadType};
use platforms::{Platform, NanoLambda, AwsLambda};
use statistics::BenchmarkStats;

#[derive(Parser, Debug)]
#[command(name = "benchmark-runner")]
#[command(about = "Benchmark NanoLambda vs AWS Lambda")]
struct Args {
    /// Platform to benchmark: nanolambda, aws-lambda, or both
    #[arg(short, long, default_value = "nanolambda")]
    platform: String,

    /// Number of warm-up invocations
    #[arg(short, long, default_value = "10")]
    warmup: usize,

    /// Number of benchmark iterations
    #[arg(short, long, default_value = "100")]
    iterations: usize,

    /// Output file for results (JSON)
    #[arg(short, long)]
    output: Option<String>,

    /// Specific workload to run (default: all)
    #[arg(short = 't', long)]
    workload_type: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Tabled)]
struct BenchmarkResult {
    platform: String,
    workload: String,
    cold_start_ms: f64,
    warm_p50_ms: f64,
    warm_p95_ms: f64,
    warm_p99_ms: f64,
    throughput_rps: f64,
    memory_mb: u64,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let args = Args::parse();

    println!("🚀 NanoLambda Benchmark Suite\n");

    // Determine which workloads to run
    let workloads = if let Some(wt) = &args.workload_type {
        vec![WorkloadType::from_str(wt)?]
    } else {
        vec![
            WorkloadType::HelloWorld,
            WorkloadType::JsonProcessing,
            WorkloadType::ComputeHeavy,
            WorkloadType::IoOperations,
        ]
    };

    let mut all_results = Vec::new();

    // Run benchmarks for each platform
    if args.platform == "nanolambda" || args.platform == "both" {
        println!("📊 Benchmarking NanoLambda...\n");
        let platform = NanoLambda::new().await?;
        let results = run_benchmarks(&platform, &workloads, &args).await?;
        all_results.extend(results);
    }

    if args.platform == "aws-lambda" || args.platform == "both" {
        println!("☁️  Benchmarking AWS Lambda...\n");
        let platform = AwsLambda::new().await?;
        let results = run_benchmarks(&platform, &workloads, &args).await?;
        all_results.extend(results);
    }

    // Display results
    display_results(&all_results);

    // Save to file if requested
    if let Some(output_path) = &args.output {
        let json = serde_json::to_string_pretty(&all_results)?;
        std::fs::write(output_path, json)?;
        println!("\n💾 Results saved to: {}", output_path);
    }

    // If comparing both platforms, show comparison table
    if args.platform == "both" {
        println!("\n🔬 Comparison Analysis\n");
        display_comparison(&all_results);
    }

    Ok(())
}

async fn run_benchmarks<P: Platform>(
    platform: &P,
    workloads: &[WorkloadType],
    args: &Args,
) -> anyhow::Result<Vec<BenchmarkResult>> {
    let mut results = Vec::new();

    for workload_type in workloads {
        let workload = Workload::new(*workload_type);
        println!("  📦 Workload: {}", workload.name());

        // Deploy function
        platform.deploy(&workload).await?;

        // Measure cold start
        println!("    ❄️  Measuring cold start...");
        let cold_start = measure_cold_start(platform, &workload).await?;
        println!("       Cold start: {:.2}ms", cold_start);

        // Warm-up phase
        println!("    🔥 Warming up ({} iterations)...", args.warmup);
        for _ in 0..args.warmup {
            platform.invoke(&workload).await?;
        }

        // Measure warm starts
        println!("    📈 Measuring warm performance ({} iterations)...", args.iterations);
        let mut warm_latencies = Vec::with_capacity(args.iterations);
        
        for _ in 0..args.iterations {
            let start = Instant::now();
            platform.invoke(&workload).await?;
            warm_latencies.push(start.elapsed().as_secs_f64() * 1000.0);
        }

        let stats = BenchmarkStats::from_latencies(&warm_latencies);

        // Measure throughput
        println!("    ⚡ Measuring throughput...");
        let throughput = measure_throughput(platform, &workload).await?;

        // Get memory usage
        let memory = platform.get_memory_usage(&workload).await?;

        let result = BenchmarkResult {
            platform: platform.name().to_string(),
            workload: workload.name().to_string(),
            cold_start_ms: cold_start,
            warm_p50_ms: stats.p50,
            warm_p95_ms: stats.p95,
            warm_p99_ms: stats.p99,
            throughput_rps: throughput,
            memory_mb: memory,
        };

        println!("    ✅ P50: {:.2}ms | P95: {:.2}ms | P99: {:.2}ms | Throughput: {:.1} req/s\n",
                 stats.p50, stats.p95, stats.p99, throughput);

        results.push(result);

        // Cleanup
        platform.cleanup(&workload).await?;
    }

    Ok(results)
}

async fn measure_cold_start<P: Platform>(
    platform: &P,
    workload: &Workload,
) -> anyhow::Result<f64> {
    // Ensure clean slate
    platform.ensure_cold_state(workload).await?;
    
    let start = Instant::now();
    platform.invoke(workload).await?;
    let duration = start.elapsed().as_secs_f64() * 1000.0;
    
    Ok(duration)
}

async fn measure_throughput<P: Platform>(
    platform: &P,
    workload: &Workload,
) -> anyhow::Result<f64> {
    let duration = Duration::from_secs(5);
    let mut count = 0;
    let start = Instant::now();

    while start.elapsed() < duration {
        platform.invoke(workload).await?;
        count += 1;
    }

    let elapsed = start.elapsed().as_secs_f64();
    Ok(count as f64 / elapsed)
}

fn display_results(results: &[BenchmarkResult]) {
    let table = Table::new(results).to_string();
    println!("\n📊 Benchmark Results\n");
    println!("{}", table);
}

fn display_comparison(results: &[BenchmarkResult]) {
    // Group by workload, compare platforms
    let mut workload_groups: std::collections::HashMap<String, Vec<&BenchmarkResult>> =
        std::collections::HashMap::new();

    for result in results {
        workload_groups
            .entry(result.workload.clone())
            .or_insert_with(Vec::new)
            .push(result);
    }

    for (workload, platforms) in workload_groups {
        if platforms.len() < 2 {
            continue;
        }

        println!("  📦 {}", workload);

        let nano = platforms.iter().find(|p| p.platform == "NanoLambda");
        let aws = platforms.iter().find(|p| p.platform == "AWS Lambda");

        if let (Some(nano), Some(aws)) = (nano, aws) {
            let cold_speedup = aws.cold_start_ms / nano.cold_start_ms;
            let warm_speedup = aws.warm_p50_ms / nano.warm_p50_ms;
            let throughput_ratio = nano.throughput_rps / aws.throughput_rps;

            println!("    Cold Start: {} ({:.1}x faster)",
                     format_speedup(cold_speedup),
                     cold_speedup);
            println!("    Warm P50:   {} ({:.1}x faster)",
                     format_speedup(warm_speedup),
                     warm_speedup);
            println!("    Throughput: {} ({:.1}x higher)",
                     format_speedup(throughput_ratio),
                     throughput_ratio);
        }
        println!();
    }
}

fn format_speedup(ratio: f64) -> String {
    let symbol = if ratio > 1.0 { "🚀" } else { "🐌" };
    let text = format!("{} {:.2}x", symbol, ratio);
    if ratio > 1.5 {
        text.green().to_string()
    } else if ratio > 1.0 {
        text.yellow().to_string()
    } else {
        text.red().to_string()
    }
}
