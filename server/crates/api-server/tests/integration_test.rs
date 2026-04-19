//! Integration tests for storage + runtime integration

use nanolambda_api::ApiServer;
use std::collections::HashMap;
use std::sync::Arc;

#[tokio::test]
async fn test_create_python_function() {
    let server = Arc::new(ApiServer::new_in_memory().await.unwrap());

    let config = nanolambda_storage::FunctionConfig {
        name: "test-python".to_string(),
        runtime: "python3.12".to_string(),
        handler: "handler".to_string(),
        code: "def handler(event, context):\n    return {'result': event.get('x', 0) + 1}"
            .to_string(),
        memory_mb: 128,
        timeout_ms: 30000,
        environment: HashMap::default(),
    };

    server.storage().create_function(config).unwrap();

    let function = server.storage().get_function("test-python").unwrap();
    assert!(function.is_some(), "Function should be created in storage");

    let func = function.unwrap();
    assert_eq!(func.name, "test-python");
    assert_eq!(func.runtime, "python3.12");
}

#[tokio::test]
async fn test_list_functions() {
    let server = Arc::new(ApiServer::new_in_memory().await.unwrap());

    let storage = server.storage();

    let config1 = nanolambda_storage::FunctionConfig {
        name: "func1".to_string(),
        runtime: "python3.12".to_string(),
        handler: "handler".to_string(),
        code: "def handler(e, c): return {'result': 'ok'}".to_string(),
        memory_mb: 128,
        timeout_ms: 30000,
        environment: HashMap::default(),
    };

    let config2 = nanolambda_storage::FunctionConfig {
        name: "func2".to_string(),
        runtime: "python3.12".to_string(),
        handler: "handler".to_string(),
        code: "def handler(e, c): return {'result': 'ok'}".to_string(),
        memory_mb: 128,
        timeout_ms: 30000,
        environment: HashMap::default(),
    };

    storage.create_function(config1).unwrap();
    storage.create_function(config2).unwrap();

    let functions = storage.list_functions().unwrap();
    assert_eq!(functions.len(), 2);
}

#[tokio::test]
async fn test_update_function() {
    let server = Arc::new(ApiServer::new_in_memory().await.unwrap());

    let config = nanolambda_storage::FunctionConfig {
        name: "update-test".to_string(),
        runtime: "python3.12".to_string(),
        handler: "handler".to_string(),
        code: "def handler(e, c): return {'version': 1}".to_string(),
        memory_mb: 128,
        timeout_ms: 30000,
        environment: HashMap::default(),
    };

    server.storage().create_function(config).unwrap();

    let updated_config = nanolambda_storage::FunctionConfig {
        name: "update-test".to_string(),
        runtime: "python3.12".to_string(),
        handler: "handler".to_string(),
        code: "def handler(e, c): return {'version': 2}".to_string(),
        memory_mb: 256,
        timeout_ms: 60000,
        environment: HashMap::default(),
    };

    server
        .storage()
        .update_function("update-test", updated_config)
        .unwrap();

    let function = server
        .storage()
        .get_function("update-test")
        .unwrap()
        .unwrap();
    assert_eq!(function.memory_mb, 256);
    assert_eq!(function.timeout_ms, 60000);
}

#[tokio::test]
async fn test_delete_function() {
    let server = Arc::new(ApiServer::new_in_memory().await.unwrap());

    let config = nanolambda_storage::FunctionConfig {
        name: "delete-test".to_string(),
        runtime: "python3.12".to_string(),
        handler: "handler".to_string(),
        code: "def handler(e, c): return {}".to_string(),
        memory_mb: 128,
        timeout_ms: 30000,
        environment: HashMap::default(),
    };

    server.storage().create_function(config).unwrap();

    server.storage().delete_function("delete-test").unwrap();

    let function = server.storage().get_function("delete-test").unwrap();
    assert!(
        function.is_none()
            || function.unwrap().status == nanolambda_storage::FunctionStatus::Deleted
    );
}

#[tokio::test]
async fn test_health_check() {
    let server = Arc::new(ApiServer::new_in_memory().await.unwrap());

    assert!(server.storage().list_functions().is_ok());

    let py_executor = server.python_executor().lock().await;
    drop(py_executor);
}
