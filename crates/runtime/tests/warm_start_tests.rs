//! Warm start benchmark tests

use nanolambda_runtime::{PythonExecutor, FunctionConfig};
use serde_json::json;
use std::collections::HashMap;
use std::time::Instant;

#[test]
fn test_warm_vs_cold_start_performance() {
    let mut executor = PythonExecutor::new().expect("Failed to create executor");
    
    let function_code = r#"
def handler(event, context):
    name = event.get('name', 'World')
    return {'message': f'Hello, {name}!'}
"#;
    
    let config = FunctionConfig {
        id: 1,
        version: 1,
        name: "benchmark-func".to_string(),
        code: function_code.to_string(),
        handler: "handler".to_string(),
        environment: HashMap::new(),
        memory_limit_mb: 128,
        timeout_seconds: 30,
        working_dir: None,
    };
    
    // Measure warm starts (default enabled)
    let mut warm_times = vec![];
    let mut warm_cold_count = 0;
    
    for i in 0..10 {
        let event = json!({"name": format!("Test{}", i)});
        let start = Instant::now();
        let result = executor.execute(config.clone(), event).expect("Execution failed");
        let duration = start.elapsed();
        
        warm_times.push(duration.as_millis());
        if result.metrics.is_cold_start {
            warm_cold_count += 1;
        }
        
        println!("Warm iteration {}: {}ms (cold_start: {})", 
            i, duration.as_millis(), result.metrics.is_cold_start);
    }
    
    // Disable warm starts and measure cold starts
    executor.disable_warm_starts();
    
    let mut cold_times = vec![];
    
    for i in 0..10 {
        let event = json!({"name": format!("Test{}", i)});
        let start = Instant::now();
        let result = executor.execute(config.clone(), event).expect("Execution failed");
        let duration = start.elapsed();
        
        cold_times.push(duration.as_millis());
        assert!(result.metrics.is_cold_start, "Should be cold start");
        
        println!("Cold iteration {}: {}ms", i, duration.as_millis());
    }
    
    // Calculate averages (excluding first iteration as it may include pool setup)
    let warm_avg = if warm_times.len() > 1 {
        warm_times[1..].iter().sum::<u128>() / (warm_times.len() - 1) as u128
    } else {
        0
    };
    
    let cold_avg = if cold_times.len() > 1 {
        cold_times[1..].iter().sum::<u128>() / (cold_times.len() - 1) as u128
    } else {
        0
    };
    
    println!("\n=== Performance Comparison ===");
    println!("Warm starts avg (iterations 2-10): {}ms", warm_avg);
    println!("Cold starts avg (iterations 2-10): {}ms", cold_avg);
    println!("Speedup: {:.2}x faster", cold_avg as f64 / warm_avg as f64);
    println!("Cold starts in warm mode: {} out of 10", warm_cold_count);
    
    // Warm starts should be significantly faster after first invocation
    // First one is cold, rest should be warm
    assert_eq!(warm_cold_count, 1, "Should have exactly 1 cold start in warm mode");
    
    // Warm starts should be at least 2x faster on average
    if warm_avg > 0 && cold_avg > 0 {
        let speedup = cold_avg as f64 / warm_avg as f64;
        assert!(speedup >= 2.0, 
            "Warm starts should be at least 2x faster (actual: {:.2}x)", speedup);
    }
}

#[test]
fn test_warm_start_consistency() {
    let executor = PythonExecutor::new().expect("Failed to create executor");
    
    let function_code = r#"
def handler(event, context):
    x = event.get('x', 0)
    y = event.get('y', 0)
    return {'sum': x + y, 'product': x * y}
"#;
    
    let config = FunctionConfig {
        id: 2,
        version: 1,
        name: "math-func".to_string(),
        code: function_code.to_string(),
        handler: "handler".to_string(),
        environment: HashMap::new(),
        memory_limit_mb: 128,
        timeout_seconds: 30,
        working_dir: None,
    };
    
    // Execute multiple times with different inputs
    for i in 1..=5 {
        let event = json!({"x": i, "y": i * 2});
        let result = executor.execute(config.clone(), event).expect("Execution failed");
        
        assert!(result.success, "Execution should succeed");
        
        let expected_sum = i + (i * 2);
        let expected_product = i * (i * 2);
        
        // Parse result to verify correctness
        if let Some(result_str) = result.result {
            println!("Iteration {}: {}", i, result_str);
            assert!(result_str.contains(&expected_sum.to_string()), 
                "Result should contain correct sum");
            assert!(result_str.contains(&expected_product.to_string()), 
                "Result should contain correct product");
        }
        
        // After first invocation, all should be warm starts
        if i > 1 {
            assert!(!result.metrics.is_cold_start, 
                "Iteration {} should be warm start", i);
        }
    }
}

#[test]
fn test_multiple_functions_isolation() {
    let executor = PythonExecutor::new().expect("Failed to create executor");
    
    let func1_code = r#"
def handler(event, context):
    return {'function': 'func1', 'value': event.get('value', 0) * 2}
"#;
    
    let func2_code = r#"
def handler(event, context):
    return {'function': 'func2', 'value': event.get('value', 0) + 10}
"#;
    
    let config1 = FunctionConfig {
        id: 3,
        version: 1,
        name: "func1".to_string(),
        code: func1_code.to_string(),
        handler: "handler".to_string(),
        environment: HashMap::new(),
        memory_limit_mb: 128,
        timeout_seconds: 30,
        working_dir: None,
    };
    
    let config2 = FunctionConfig {
        id: 4,
        version: 1,
        name: "func2".to_string(),
        code: func2_code.to_string(),
        handler: "handler".to_string(),
        environment: HashMap::new(),
        memory_limit_mb: 128,
        timeout_seconds: 30,
        working_dir: None,
    };
    
    // Invoke both functions alternately
    for i in 0..3 {
        let event = json!({"value": i});
        
        let result1 = executor.execute(config1.clone(), event.clone())
            .expect("Func1 execution failed");
        let result2 = executor.execute(config2.clone(), event.clone())
            .expect("Func2 execution failed");
        
        assert!(result1.success && result2.success, "Both should succeed");
        
        if let (Some(r1), Some(r2)) = (result1.result, result2.result) {
            println!("Iteration {}: func1={}, func2={}", i, r1, r2);
            assert!(r1.contains("func1") && r1.contains(&(i * 2).to_string()));
            assert!(r2.contains("func2") && r2.contains(&(i + 10).to_string()));
        }
    }
}
