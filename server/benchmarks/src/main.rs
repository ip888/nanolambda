use clap::Parser;
use colored::Colorize;
use serde::{Deserialize, Serialize};
use std::path::Path;
use std::time::{Duration, Instant};
use tabled::{Table, Tabled};

mod e2b;
mod platforms;
mod report;
mod statistics;
mod workloads;

use e2b::E2B;
use platforms::{AwsLambda, NanoLambda, Platform};
use statistics::BenchmarkStats;
use workloads::{Workload, WorkloadType};

#[derive(Parser, Debug)]
#[command(name = "benchmark-runner")]
#[command(about = "Benchmark NanoLambda vs AWS Lambda")]
struct Args {
    /// Platform to benchmark: nanolambda, aws-lambda, e2b, or all
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
pub struct BenchmarkResult {
    pub platform: String,
    pub workload: String,
    pub cold_start_ms: f64,
    pub warm_p50_ms: f64,
    pub warm_p95_ms: f64,
    pub warm_p99_ms: f64,
    pub throughput_rps: f64,
    pub memory_mb: u64,
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

    let run_nanolambda = matches!(args.platform.as_str(), "nanolambda" | "both" | "all");
    let run_aws = matches!(args.platform.as_str(), "aws-lambda" | "both" | "all");
    let run_e2b = matches!(args.platform.as_str(), "e2b" | "all");

    if run_nanolambda {
        println!("📊 Benchmarking NanoLambda...\n");
        let platform = NanoLambda::new().await?;
        let results = run_benchmarks(&platform, &workloads, &args).await?;
        all_results.extend(results);
    }

    if run_aws {
        println!("☁️  Benchmarking AWS Lambda...\n");
        let platform = AwsLambda::new().await?;
        let results = run_benchmarks(&platform, &workloads, &args).await?;
        all_results.extend(results);
    }

    if run_e2b {
        println!("🧪 Benchmarking E2B...\n");
        match E2B::new() {
            Ok(platform) => {
                let results = run_benchmarks(&platform, &workloads, &args).await?;
                all_results.extend(results);
            }
            Err(err) => {
                // Don't tank the whole run — just skip E2B when creds missing.
                eprintln!("⚠️  Skipping E2B: {err}");
            }
        }
    }

    display_results(&all_results);

    if let Some(output_path) = &args.output {
        let json = serde_json::to_string_pretty(&all_results)?;
        std::fs::write(output_path, json)?;
        println!("\n💾 Results saved to: {}", output_path);

        // Always emit a sibling Markdown report for the benchmark blog post.
        let md_path = Path::new(output_path).with_extension("md");
        let md = report::render_markdown(&all_results);
        std::fs::write(&md_path, md)?;
        println!("📝 Markdown report: {}", md_path.display());
    }

    if matches!(args.platform.as_str(), "both" | "all") {
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
        println!(
            "    📈 Measuring warm performance ({} iterations)...",
            args.iterations
        );
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

        println!(
            "    ✅ P50: {:.2}ms | P95: {:.2}ms | P99: {:.2}ms | Throughput: {:.1} req/s\n",
            stats.p50, stats.p95, stats.p99, throughput
        );

        results.push(result);

        // Cleanup
        platform.cleanup(&workload).await?;
    }

    Ok(results)
}

async fn measure_cold_start<P: Platform>(platform: &P, workload: &Workload) -> anyhow::Result<f64> {
    // Ensure clean slate
    platform.ensure_cold_state(workload).await?;

    let start = Instant::now();
    platform.invoke(workload).await?;
    let duration = start.elapsed().as_secs_f64() * 1000.0;

    Ok(duration)
}

async fn measure_throughput<P: Platform>(platform: &P, workload: &Workload) -> anyhow::Result<f64> {
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
            .or_default()
            .push(result);
    }

    for (workload, platforms) in workload_groups {
        if platforms.len() < 2 {
            continue;
        }

        println!("  📦 {}", workload);

        let nano = platforms.iter().find(|p| p.platform == "NanoLambda");

        for other in platforms.iter().filter(|p| p.platform != "NanoLambda") {
            let Some(nano) = nano else { continue };
            let cold_speedup = other.cold_start_ms / nano.cold_start_ms;
            let warm_speedup = other.warm_p50_ms / nano.warm_p50_ms;
            let throughput_ratio = nano.throughput_rps / other.throughput_rps;

            println!("    vs {}", other.platform);
            println!(
                "      Cold Start: {} ({:.1}x faster)",
                format_speedup(cold_speedup),
                cold_speedup
            );
            println!(
                "      Warm P50:   {} ({:.1}x faster)",
                format_speedup(warm_speedup),
                warm_speedup
            );
            println!(
                "      Throughput: {} ({:.1}x higher)",
                format_speedup(throughput_ratio),
                throughput_ratio
            );
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
