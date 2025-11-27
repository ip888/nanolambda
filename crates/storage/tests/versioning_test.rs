//! Test function versioning functionality

use nanolambda_storage::{StorageManager, FunctionConfig};
use std::collections::HashMap;

#[test]
fn test_function_versioning_workflow() {
    // Create storage manager
    let storage = StorageManager::new_in_memory().expect("Failed to create storage");

    // Create initial function (v1)
    let config_v1 = FunctionConfig {
        name: "test-func".to_string(),
        runtime: "python3.11".to_string(),
        handler: "handler".to_string(),
        code: "def handler(event, context): return {'version': 1}".to_string(),
        memory_mb: 128,
        timeout_ms: 3000,
        environment: HashMap::new(),
    };

    let func_id = storage.create_function(config_v1.clone()).expect("Failed to create function");
    println!("Created function with id: {}", func_id);

    // Verify v1 exists and is latest
    let func_v1 = storage.get_function("test-func").expect("Failed to get function").expect("Function not found");
    assert_eq!(func_v1.version, 1);
    assert_eq!(func_v1.is_latest, true);
    assert!(func_v1.code.contains("version': 1"));

    // Publish v2 with different code
    let config_v2 = FunctionConfig {
        name: "test-func".to_string(),
        runtime: "python3.11".to_string(),
        handler: "handler".to_string(),
        code: "def handler(event, context): return {'version': 2}".to_string(),
        memory_mb: 128,
        timeout_ms: 3000,
        environment: HashMap::new(),
    };

    let func_v2_id = storage.publish_version("test-func", config_v2).expect("Failed to publish v2");
    println!("Published v2 with id: {}", func_v2_id);

    // Verify v2 is now latest
    let func_latest = storage.get_function("test-func").expect("Failed to get function").expect("Function not found");
    assert_eq!(func_latest.id, func_v2_id);
    assert_eq!(func_latest.version, 2);
    assert_eq!(func_latest.is_latest, true);
    assert!(func_latest.code.contains("version': 2"));

    // Verify v1 still exists but is not latest
    let func_v1_specific = storage.get_function_by_version("test-func", 1)
        .expect("Failed to get v1")
        .expect("v1 not found");
    assert_eq!(func_v1_specific.id, func_id);
    assert_eq!(func_v1_specific.version, 1);
    assert_eq!(func_v1_specific.is_latest, false);
    assert!(func_v1_specific.code.contains("version': 1"));

    // List all versions
    let versions = storage.list_function_versions("test-func").expect("Failed to list versions");
    assert_eq!(versions.len(), 2);
    assert_eq!(versions[0].version, 2); // Ordered DESC
    assert_eq!(versions[1].version, 1);

    // Publish v3
    let config_v3 = FunctionConfig {
        name: "test-func".to_string(),
        runtime: "python3.11".to_string(),
        handler: "handler".to_string(),
        code: "def handler(event, context): return {'version': 3}".to_string(),
        memory_mb: 128,
        timeout_ms: 3000,
        environment: HashMap::new(),
    };

    let func_v3_id = storage.publish_version("test-func", config_v3).expect("Failed to publish v3");
    println!("Published v3 with id: {}", func_v3_id);

    // Verify v3 is latest
    let func_latest = storage.get_function("test-func").expect("Failed to get function").expect("Function not found");
    assert_eq!(func_latest.version, 3);
    assert_eq!(func_latest.is_latest, true);

    // Verify all 3 versions exist
    let versions = storage.list_function_versions("test-func").expect("Failed to list versions");
    assert_eq!(versions.len(), 3);

    println!("✓ Function versioning workflow test passed!");
}

#[test]
fn test_version_isolation() {
    let storage = StorageManager::new_in_memory().expect("Failed to create storage");

    // Create function v1
    let config_v1 = FunctionConfig {
        name: "math-func".to_string(),
        runtime: "python3.11".to_string(),
        handler: "handler".to_string(),
        code: "def handler(event, context): return event['x'] * 2".to_string(),
        memory_mb: 128,
        timeout_ms: 3000,
        environment: HashMap::new(),
    };

    let id_v1 = storage.create_function(config_v1).expect("Failed to create v1");

    // Publish v2 with different logic
    let config_v2 = FunctionConfig {
        name: "math-func".to_string(),
        runtime: "python3.11".to_string(),
        handler: "handler".to_string(),
        code: "def handler(event, context): return event['x'] * 3".to_string(),
        memory_mb: 128,
        timeout_ms: 3000,
        environment: HashMap::new(),
    };

    let id_v2 = storage.publish_version("math-func", config_v2).expect("Failed to publish v2");

    // Verify IDs are different
    assert_ne!(id_v1, id_v2, "v1 and v2 should have different IDs");

    // Verify both versions have different code
    let v1 = storage.get_function_by_version("math-func", 1)
        .expect("Failed to get v1")
        .expect("v1 not found");
    let v2 = storage.get_function_by_version("math-func", 2)
        .expect("Failed to get v2")
        .expect("v2 not found");

    assert!(v1.code.contains("* 2"));
    assert!(v2.code.contains("* 3"));
    assert_ne!(v1.code_hash, v2.code_hash, "Code hashes should be different");

    println!("✓ Version isolation test passed!");
}
