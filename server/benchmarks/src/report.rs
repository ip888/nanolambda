//! Markdown report generation for the benchmark runner.
//!
//! The same JSON blob that gets written to disk can be re-rendered as a
//! Markdown table — useful for dropping straight into the benchmark blog post
//! without manual formatting.

use std::collections::BTreeMap;
use std::fmt::Write as _;

use crate::BenchmarkResult;

/// Render results grouped by workload, with platforms as rows per block.
pub fn render_markdown(results: &[BenchmarkResult]) -> String {
    let mut out = String::new();
    out.push_str("# NanoLambda benchmark report\n\n");
    out.push_str(
        "Reproducible head-to-head. Numbers are milliseconds unless otherwise noted. \
         Cold start is a fresh deploy; warm latencies are measured after the warm-up loop.\n\n",
    );

    let mut by_workload: BTreeMap<&str, Vec<&BenchmarkResult>> = BTreeMap::new();
    for r in results {
        by_workload.entry(r.workload.as_str()).or_default().push(r);
    }

    for (workload, rows) in by_workload {
        let _ = writeln!(out, "## {workload}\n");
        out.push_str(
            "| Platform | Cold (ms) | P50 (ms) | P95 (ms) | P99 (ms) | RPS | Memory (MB) |\n",
        );
        out.push_str(
            "|---|---:|---:|---:|---:|---:|---:|\n",
        );
        for r in rows {
            let _ = writeln!(
                out,
                "| {} | {:.2} | {:.2} | {:.2} | {:.2} | {:.1} | {} |",
                r.platform,
                r.cold_start_ms,
                r.warm_p50_ms,
                r.warm_p95_ms,
                r.warm_p99_ms,
                r.throughput_rps,
                r.memory_mb
            );
        }
        out.push('\n');
    }

    out.push_str("---\n\n");
    out.push_str(
        "Raw data lives alongside this report as `results.json`. Re-run with \
         `cargo run --release -p nanolambda-benchmarks -- --platform both --iterations 200`.\n",
    );
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample(platform: &str, workload: &str, cold: f64) -> BenchmarkResult {
        BenchmarkResult {
            platform: platform.into(),
            workload: workload.into(),
            cold_start_ms: cold,
            warm_p50_ms: 1.0,
            warm_p95_ms: 2.0,
            warm_p99_ms: 3.0,
            throughput_rps: 1000.0,
            memory_mb: 128,
        }
    }

    #[test]
    fn groups_by_workload() {
        let results = vec![
            sample("NanoLambda", "Hello World", 10.0),
            sample("E2B", "Hello World", 200.0),
            sample("NanoLambda", "Compute Heavy", 15.0),
        ];
        let md = render_markdown(&results);
        assert!(md.contains("## Hello World"));
        assert!(md.contains("## Compute Heavy"));
        assert!(md.contains("NanoLambda"));
        assert!(md.contains("E2B"));
    }

    #[test]
    fn platforms_render_as_table_rows() {
        let md = render_markdown(&[sample("NanoLambda", "Hello World", 10.0)]);
        assert!(md.contains("| NanoLambda |"));
        assert!(md.contains("| 10.00 |"));
    }
}
