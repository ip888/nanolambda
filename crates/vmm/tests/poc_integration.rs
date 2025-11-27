//! Integration tests for POC implementation
//! Run with: cargo test --test poc_integration

use nanolambda_vmm::{
    Vm, VmConfig,
    MemoryManager, MemoryConfig,
    PythonRuntime, PythonFunctionConfig,
};
use nanolambda_vmm::vm_poc::VmState;

#[test]
fn test_full_vm_lifecycle() {
    // 1. Create and verify VM
    let config = VmConfig::default();
    let vm = Vm::new(config).expect("Failed to create VM");
    assert_eq!(vm.state(), VmState::Created);

    // 2. Verify memory allocation
    let mem_config = MemoryConfig::default();
    let mm = MemoryManager::new(mem_config).expect("Failed to create memory manager");
    assert_eq!(mm.size(), 128 * 1024 * 1024); // 128MB
}

#[test]
fn test_python_function_execution() {
    // Create Python runtime
    let mut runtime = PythonRuntime::new().expect("Failed to create Python runtime");

    // Test function that adds two numbers
    let function = PythonFunctionConfig {
        code: r#"
def handler(a, b):
    return {"result": a + b}
        "#.to_string(),
        handler: "handler".to_string(),
        args: Some("[5, 3]".to_string()),
    };

    // Execute and verify
    let result = runtime.execute_function(function).expect("Failed to execute function");
    assert!(result.contains(r#""result": 8"#));
}

#[test]
fn test_memory_operations() {
    let mm = MemoryManager::new(MemoryConfig::default()).expect("Failed to create memory manager");
    
    // Test data
    let test_data = b"Hello from VM memory!";
    let addr = vm_memory::GuestAddress(0x1000);
    
    // Write to memory
    mm.write_slice(test_data, addr).expect("Failed to write to memory");
    
    // Read back and verify
    let mut read_buffer = vec![0u8; test_data.len()];
    mm.read_slice(&mut read_buffer, addr).expect("Failed to read from memory");
    
    assert_eq!(&read_buffer, test_data);
}