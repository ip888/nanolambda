//! Integration tests for Task 7 - Storage + Runtime Integration

use nanolambda_api::ApiServer;
use serde_json::json;
use std::sync::Arc;

#[tokio::test]
async fn test_create_python_function() {
    let server = Arc::new(ApiServer::new_in_memory().await.unwrap());

    // Create a simple Python function directly via storage
    let config = nanolambda_storage::FunctionConfig {
        name: "test-python".to_string(),
        runtime: "python3.12".to_string(),
        handler: "handler".to_string(),
        code: "def handler(event, context):\n    return {'result': event.get('x', 0) + 1}"
            .to_string(),
        memory_mb: 128,
        timeout_ms: 30000,
        environment: Default::default(),
    };

    server.storage().create_function(config).unwrap();

    println!("Test: Create Python function - SETUP COMPLETE");
    println!("  Function: test-python");
    println!("  Runtime: python3.12");
    println!("  Handler: handler");

    // Verify function was stored
    let function = server.storage().get_function("test-python").unwrap();
    assert!(function.is_some(), "Function should be created in storage");

    let func = function.unwrap();
    assert_eq!(func.name, "test-python");
    assert_eq!(func.runtime, "python3.12");
    println!("✓ Function stored successfully in database");
}

#[tokio::test]
async fn test_create_nodejs_function() {
    let server = Arc::new(ApiServer::new_in_memory().await.unwrap());

    // Create a simple Node.js function directly via storage
    let config = nanolambda_storage::FunctionConfig {
        name: "test-nodejs".to_string(),
        runtime: "nodejs20.x".to_string(),
        handler: "handler".to_string(),
        code: "exports.handler = async (event) => { return { message: `Hello ${event.name || 'World'}` }; };".to_string(),
        memory_mb: 128,
        timeout_ms: 30000,
        environment: Default::default(),
    };

    server.storage().create_function(config).unwrap();

    println!("Test: Create Node.js function - SETUP COMPLETE");
    println!("  Function: test-nodejs");
    println!("  Runtime: nodejs20.x");
    println!("  Handler: handler");

    // Verify storage works
    let function = server.storage().get_function("test-nodejs").unwrap();
    assert!(function.is_some(), "Function should be created in storage");
    println!("✓ Storage layer operational");
}

#[tokio::test]
async fn test_list_functions() {
    let server = Arc::new(ApiServer::new_in_memory().await.unwrap());

    // Create multiple functions
    let storage = server.storage();

    let config1 = nanolambda_storage::FunctionConfig {
        name: "func1".to_string(),
        runtime: "python3.12".to_string(),
        handler: "handler".to_string(),
        code: "def handler(e, c): return {'result': 'ok'}".to_string(),
        memory_mb: 128,
        timeout_ms: 30000,
        environment: Default::default(),
    };

    let config2 = nanolambda_storage::FunctionConfig {
        name: "func2".to_string(),
        runtime: "nodejs20.x".to_string(),
        handler: "handler".to_string(),
        code: "exports.handler = async (e) => ({ result: 'ok' });".to_string(),
        memory_mb: 128,
        timeout_ms: 30000,
        environment: Default::default(),
    };

    storage.create_function(config1).unwrap();
    storage.create_function(config2).unwrap();

    // List all functions
    let functions = storage.list_functions().unwrap();
    assert_eq!(functions.len(), 2);
    println!("✓ Listed {} functions from storage", functions.len());
}

#[tokio::test]
async fn test_update_function() {
    let server = Arc::new(ApiServer::new_in_memory().await.unwrap());

    // Create initial function
    let config = nanolambda_storage::FunctionConfig {
        name: "update-test".to_string(),
        runtime: "python3.12".to_string(),
        handler: "handler".to_string(),
        code: "def handler(e, c): return {'version': 1}".to_string(),
        memory_mb: 128,
        timeout_ms: 30000,
        environment: Default::default(),
    };

    server.storage().create_function(config).unwrap();

    // Update function
    let updated_config = nanolambda_storage::FunctionConfig {
        name: "update-test".to_string(),
        runtime: "python3.12".to_string(),
        handler: "handler".to_string(),
        code: "def handler(e, c): return {'version': 2}".to_string(),
        memory_mb: 256,
        timeout_ms: 60000,
        environment: Default::default(),
    };

    server
        .storage()
        .update_function("update-test", updated_config)
        .unwrap();

    // Verify update
    let function = server
        .storage()
        .get_function("update-test")
        .unwrap()
        .unwrap();
    assert_eq!(function.memory_mb, 256);
    assert_eq!(function.timeout_ms, 60000);
    println!("✓ Function updated successfully");
}

#[tokio::test]
async fn test_delete_function() {
    let server = Arc::new(ApiServer::new_in_memory().await.unwrap());

    // Create function
    let config = nanolambda_storage::FunctionConfig {
        name: "delete-test".to_string(),
        runtime: "python3.12".to_string(),
        handler: "handler".to_string(),
        code: "def handler(e, c): return {}".to_string(),
        memory_mb: 128,
        timeout_ms: 30000,
        environment: Default::default(),
    };

    server.storage().create_function(config).unwrap();

    // Delete function
    server.storage().delete_function("delete-test").unwrap();

    // Verify deletion
    let function = server.storage().get_function("delete-test").unwrap();
    assert!(
        function.is_none()
            || function.unwrap().status == nanolambda_storage::FunctionStatus::Deleted
    );
    println!("✓ Function deleted successfully");
}

#[tokio::test]
async fn test_health_check() {
    let server = Arc::new(ApiServer::new_in_memory().await.unwrap());

    // Verify all components are initialized
    assert!(server.storage().list_functions().is_ok());
    println!("✓ Storage layer: Healthy");

    // Python executor should be available
    let py_executor = server.python_executor().lock().await;
    drop(py_executor);
    println!("✓ Python executor: Healthy");

    // Node.js executor should be available
    let node_executor = server.nodejs_executor().lock().await;
    drop(node_executor);
    println!("✓ Node.js executor: Healthy");

    println!("✓ All components healthy - API server ready");
}
