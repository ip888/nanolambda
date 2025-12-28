//! Demonstration of memory and CPU tracking in NanoLambda
//!
//! This example shows how the runtime now tracks real memory usage
//! from the Linux /proc filesystem instead of using placeholder values.
//!
//! Run with: cargo run --example memory_tracking_demo

use nanolambda_runtime::{FunctionConfig, PythonExecutor};
use std::collections::HashMap;
use std::path::PathBuf;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== NanoLambda Memory Tracking Demo ===\n");

    // Create executor with warm starts enabled
    let mut executor = PythonExecutor::with_base_dir(PathBuf::from("/tmp/nanolambda-demo"))?;
    executor.enable_warm_starts();

    // Define a memory-intensive function
    let memory_heavy_code = r#"
def handler(event, context):
    # Allocate some memory
    size_mb = event.get('size_mb', 10)
    data = [0] * (size_mb * 1024 * 128)  # Allocate ~size_mb MB
    
    return {
        'allocated_mb': size_mb,
        'list_length': len(data)
    }
"#;

    let config = FunctionConfig {
        id: 1,
        version: 1,
        name: "memory-test".to_string(),
        code: memory_heavy_code.to_string(),
        handler: "handler".to_string(),
        environment: HashMap::new(),
        memory_limit_mb: 512,
        timeout_seconds: 30,
        working_dir: None,
    };

    // Run multiple invocations with different memory sizes
    println!("Running memory-intensive function with different sizes:\n");

    for size_mb in [5, 10, 20, 30] {
        let event = serde_json::json!({
            "size_mb": size_mb
        });

        let result = executor.execute(config.clone(), event)?;

        println!("Allocation Size: {} MB", size_mb);
        println!("  Success: {}", result.success);
        println!("  Execution Time: {} ms", result.metrics.execution_ms);
        println!("  Peak Memory: {:.2} MB", result.metrics.memory_peak_mb);
        println!("  CPU Usage: {:.1}%", result.metrics.cpu_percent);
        println!("  Cold Start: {}", result.metrics.is_cold_start);

        if let Some(res) = result.result {
            println!("  Result: {}", res);
        }

        println!();
    }

    // Test CPU-intensive function
    let cpu_heavy_code = r#"
def handler(event, context):
    # CPU-intensive computation
    iterations = event.get('iterations', 100000)
    result = 0
    for i in range(iterations):
        result += i * i
    
    return {
        'iterations': iterations,
        'result': result
    }
"#;

    let cpu_config = FunctionConfig {
        id: 2,
        version: 1,
        name: "cpu-test".to_string(),
        code: cpu_heavy_code.to_string(),
        handler: "handler".to_string(),
        environment: HashMap::new(),
        memory_limit_mb: 256,
        timeout_seconds: 30,
        working_dir: None,
    };

    println!("\nRunning CPU-intensive function:\n");

    for iterations in [100_000, 500_000, 1_000_000] {
        let event = serde_json::json!({
            "iterations": iterations
        });

        let result = executor.execute(cpu_config.clone(), event)?;

        println!("Iterations: {}", iterations);
        println!("  Execution Time: {} ms", result.metrics.execution_ms);
        println!("  Peak Memory: {:.2} MB", result.metrics.memory_peak_mb);
        println!("  CPU Usage: {:.1}%", result.metrics.cpu_percent);
        println!("  Cold Start: {}", result.metrics.is_cold_start);
        println!();
    }

    println!("=== Demo Complete ===");
    println!("\nKey Features:");
    println!("✓ Real memory tracking from /proc filesystem");
    println!("✓ RSS (Resident Set Size) - actual RAM used");
    println!("✓ Peak memory usage tracking");
    println!("✓ CPU usage percentage");
    println!("✓ Cold vs warm start detection");

    Ok(())
}
