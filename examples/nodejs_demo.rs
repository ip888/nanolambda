//! Node.js Runtime Example
//!
//! Demonstrates the Node.js runtime with various function types.
//!
//! Run with: cargo run --example nodejs_demo

use nanolambda_runtime::{GenericFunctionConfig, Language, NodeJSExecutor, Runtime};
use serde_json::json;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== Node.js Runtime Demo ===\n");

    // Create Node.js executor
    let mut executor = NodeJSExecutor::new()?;
    println!("✓ Node.js executor created");
    
    // Get runtime info
    let info = executor.runtime_info();
    println!("  Language: {:?}", info.language);
    println!("  Version: {}", info.version);
    println!("  Path: {}", info.interpreter_path.display());
    println!("  Warm starts: {}", info.capabilities.warm_starts);
    println!();

    // Example 1: Simple synchronous function
    println!("1. Simple Synchronous Function");
    println!("   -------------------------------");
    let simple_config = GenericFunctionConfig::new(
        "simple-function".to_string(),
        Language::NodeJS,
        r#"
            exports.handler = (event) => {
                return {
                    message: `Hello, ${event.name}!`,
                    timestamp: new Date().toISOString()
                };
            };
        "#
        .to_string(),
        "handler".to_string(),
    );

    let result = executor
        .execute(&simple_config, json!({ "name": "World" }))
        .await?;
    println!("   Success: {}", result.success);
    if let Some(output) = &result.result {
        let parsed: serde_json::Value = serde_json::from_str(output)?;
        println!("   Output: {}", serde_json::to_string_pretty(&parsed)?);
    }
    println!("   Execution time: {}ms", result.metrics.execution_ms);
    println!("   Cold start: {}", result.metrics.is_cold_start);
    println!();

    // Example 2: Async function with Promise
    println!("2. Async Function with Promise");
    println!("   -----------------------------");
    let async_config = GenericFunctionConfig::new(
        "async-function".to_string(),
        Language::NodeJS,
        r#"
            exports.handler = async (event) => {
                // Simulate async operation
                await new Promise(resolve => setTimeout(resolve, 100));
                
                return {
                    processed: true,
                    input: event.data,
                    result: event.data * 2
                };
            };
        "#
        .to_string(),
        "handler".to_string(),
    );

    let result = executor
        .execute(&async_config, json!({ "data": 42 }))
        .await?;
    println!("   Success: {}", result.success);
    if let Some(output) = &result.result {
        let parsed: serde_json::Value = serde_json::from_str(output)?;
        println!("   Output: {}", serde_json::to_string_pretty(&parsed)?);
    }
    println!("   Execution time: {}ms", result.metrics.execution_ms);
    println!();

    // Example 3: Function with state (warm starts)
    println!("3. Stateful Function (Warm Starts)");
    println!("   --------------------------------");
    executor.set_warm_starts(true);

    let stateful_config = GenericFunctionConfig::new(
        "stateful-function".to_string(),
        Language::NodeJS,
        r#"
            let invocationCount = 0;
            let startTime = Date.now();

            exports.handler = (event) => {
                invocationCount++;
                const uptime = Date.now() - startTime;
                
                return {
                    invocation: invocationCount,
                    uptime_ms: uptime,
                    message: event.message
                };
            };
        "#
        .to_string(),
        "handler".to_string(),
    );

    // First invocation (cold start)
    let result1 = executor
        .execute(&stateful_config, json!({ "message": "First call" }))
        .await?;
    println!("   Invocation 1:");
    if let Some(output) = &result1.result {
        let parsed: serde_json::Value = serde_json::from_str(output)?;
        println!("     Output: {}", serde_json::to_string_pretty(&parsed)?);
    }
    println!("     Cold start: {}", result1.metrics.is_cold_start);
    println!("     Time: {}ms", result1.metrics.total_ms);

    // Second invocation (warm start)
    let result2 = executor
        .execute(&stateful_config, json!({ "message": "Second call" }))
        .await?;
    println!("   Invocation 2:");
    if let Some(output) = &result2.result {
        let parsed: serde_json::Value = serde_json::from_str(output)?;
        println!("     Output: {}", serde_json::to_string_pretty(&parsed)?);
    }
    println!("     Cold start: {}", result2.metrics.is_cold_start);
    let speedup = if result2.metrics.total_ms > 0 {
        format!("{}x faster", result1.metrics.total_ms / result2.metrics.total_ms.max(1))
    } else {
        "< 1ms (very fast!)".to_string()
    };
    println!("     Time: {}ms ({})", result2.metrics.total_ms, speedup);
    println!();

    // Example 4: Error handling
    println!("4. Error Handling");
    println!("   --------------");
    let error_config = GenericFunctionConfig::new(
        "error-function".to_string(),
        Language::NodeJS,
        r#"
            exports.handler = (event) => {
                if (event.shouldError) {
                    throw new Error('This is a test error');
                }
                return { success: true };
            };
        "#
        .to_string(),
        "handler".to_string(),
    );

    let result = executor
        .execute(&error_config, json!({ "shouldError": true }))
        .await?;
    println!("   Success: {}", result.success);
    if let Some(error) = &result.error {
        let lines: Vec<&str> = error.lines().take(3).collect();
        println!("   Error: {}", lines.join("\n          "));
    }
    println!();

    // Example 5: Complex data processing
    println!("5. Complex Data Processing");
    println!("   -----------------------");
    let complex_config = GenericFunctionConfig::new(
        "data-processor".to_string(),
        Language::NodeJS,
        r#"
            exports.handler = (event) => {
                const items = event.items || [];
                
                const stats = {
                    count: items.length,
                    sum: items.reduce((a, b) => a + b, 0),
                    average: items.length > 0 ? items.reduce((a, b) => a + b, 0) / items.length : 0,
                    max: items.length > 0 ? Math.max(...items) : null,
                    min: items.length > 0 ? Math.min(...items) : null
                };
                
                return {
                    input_count: items.length,
                    statistics: stats,
                    processed_at: new Date().toISOString()
                };
            };
        "#
        .to_string(),
        "handler".to_string(),
    );

    let numbers = vec![1, 5, 3, 9, 2, 7, 4, 8, 6];
    let result = executor
        .execute(&complex_config, json!({ "items": numbers }))
        .await?;
    println!("   Input: {:?}", numbers);
    if let Some(output) = &result.result {
        let parsed: serde_json::Value = serde_json::from_str(output)?;
        println!("   Output: {}", serde_json::to_string_pretty(&parsed)?);
    }
    println!("   Execution time: {}ms", result.metrics.execution_ms);
    println!("   Memory used: {:.2} MB", result.metrics.memory_peak_mb);
    println!();

    // Health check
    println!("6. Health Check");
    println!("   ------------");
    match executor.health_check() {
        Ok(_) => println!("   ✓ Node.js runtime is healthy"),
        Err(e) => println!("   ✗ Health check failed: {}", e),
    }
    println!();

    println!("=== Demo Complete ===");

    Ok(())
}
