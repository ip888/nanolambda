use nanolambda_storage::{CreateApiKeyRequest, StorageManager, ApiKeyStatus};

#[test]
fn test_create_api_key() {
    let storage = StorageManager::new_in_memory().unwrap();
    
    let request = CreateApiKeyRequest {
        name: "test-key".to_string(),
        permissions: vec!["functions:create".to_string(), "functions:invoke".to_string()],
        expires_at: None,
    };
    
    let key = storage.create_api_key(request).unwrap();
    
    assert!(key.key.starts_with("nl_"));
    assert_eq!(key.name, "test-key");
    assert_eq!(key.permissions.len(), 2);
    assert_eq!(key.status, ApiKeyStatus::Active);
    assert!(key.last_used_at.is_none());
}

#[test]
fn test_get_api_key() {
    let storage = StorageManager::new_in_memory().unwrap();
    
    let request = CreateApiKeyRequest {
        name: "test-key".to_string(),
        permissions: vec!["functions:invoke".to_string()],
        expires_at: None,
    };
    
    let created_key = storage.create_api_key(request).unwrap();
    
    // Get by key string
    let retrieved_key = storage.get_api_key(&created_key.key).unwrap().unwrap();
    
    assert_eq!(retrieved_key.id, created_key.id);
    assert_eq!(retrieved_key.key, created_key.key);
    assert_eq!(retrieved_key.name, created_key.name);
    assert_eq!(retrieved_key.permissions, created_key.permissions);
}

#[test]
fn test_list_api_keys() {
    let storage = StorageManager::new_in_memory().unwrap();
    
    // Create multiple keys
    for i in 1..=3 {
        let request = CreateApiKeyRequest {
            name: format!("key-{}", i),
            permissions: vec!["functions:invoke".to_string()],
            expires_at: None,
        };
        storage.create_api_key(request).unwrap();
    }
    
    let keys = storage.list_api_keys().unwrap();
    
    assert_eq!(keys.len(), 3);
    // All three keys should be present
    let names: Vec<String> = keys.iter().map(|k| k.name.clone()).collect();
    assert!(names.contains(&"key-1".to_string()));
    assert!(names.contains(&"key-2".to_string()));
    assert!(names.contains(&"key-3".to_string()));
}

#[test]
fn test_revoke_api_key() {
    let storage = StorageManager::new_in_memory().unwrap();
    
    let request = CreateApiKeyRequest {
        name: "test-key".to_string(),
        permissions: vec![],
        expires_at: None,
    };
    
    let key = storage.create_api_key(request).unwrap();
    
    // Revoke the key
    storage.revoke_api_key(key.id).unwrap();
    
    // Get the key and check status
    let revoked_key = storage.get_api_key(&key.key).unwrap().unwrap();
    assert_eq!(revoked_key.status, ApiKeyStatus::Revoked);
}

#[test]
fn test_update_last_used() {
    let storage = StorageManager::new_in_memory().unwrap();
    
    let request = CreateApiKeyRequest {
        name: "test-key".to_string(),
        permissions: vec![],
        expires_at: None,
    };
    
    let key = storage.create_api_key(request).unwrap();
    assert!(key.last_used_at.is_none());
    
    // Update last used
    storage.update_api_key_last_used(&key.key).unwrap();
    
    // Get the key and check last_used_at
    let updated_key = storage.get_api_key(&key.key).unwrap().unwrap();
    assert!(updated_key.last_used_at.is_some());
}

#[test]
fn test_api_key_uniqueness() {
    let storage = StorageManager::new_in_memory().unwrap();
    
    let request1 = CreateApiKeyRequest {
        name: "key-1".to_string(),
        permissions: vec![],
        expires_at: None,
    };
    
    let request2 = CreateApiKeyRequest {
        name: "key-2".to_string(),
        permissions: vec![],
        expires_at: None,
    };
    
    let key1 = storage.create_api_key(request1).unwrap();
    let key2 = storage.create_api_key(request2).unwrap();
    
    // Keys should be different
    assert_ne!(key1.key, key2.key);
}

#[test]
fn test_api_key_with_expiration() {
    let storage = StorageManager::new_in_memory().unwrap();
    
    let expires_at = 1735689600; // 2025-01-01 00:00:00 UTC
    
    let request = CreateApiKeyRequest {
        name: "expiring-key".to_string(),
        permissions: vec!["functions:invoke".to_string()],
        expires_at: Some(expires_at),
    };
    
    let key = storage.create_api_key(request).unwrap();
    
    assert_eq!(key.expires_at, Some(expires_at));
}
